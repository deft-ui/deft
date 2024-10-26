use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Error};
use quick_js::{JsValue, ResourceValue};
use serde::{Deserialize, Serialize};

use crate::base::{Event, EventRegistration};
use crate::event_loop::run_on_event_loop;
use crate::ext::audio_player::{AudioCurrentChangeInfo, AudioMeta, AudioNotify, AudioServer, AudioSources};
use crate::ext::common::create_event_handler;
use crate::js::js_value_util::{FromJsValue, ToJsValue};
use crate::{define_ref_and_resource};

thread_local! {
    pub static NEXT_ID: Cell<u32> = Cell::new(1);
    pub static PLAYING_MAP: RefCell<HashMap<u32, AudioResource >> = RefCell::new(HashMap::new());
    pub static PLAYER: AudioServer = AudioServer::new(handle_play_notify);
}

pub struct Audio {
    id: u32,
    event_registration: EventRegistration<AudioResource>,
    sources: Arc<Mutex<AudioSources>>,
}

impl Audio {
    pub fn new(id: u32, sources: Arc<Mutex<AudioSources>>) -> Self {
        Self {
            id,
            event_registration: EventRegistration::new(),
            sources,
        }
    }
}

define_ref_and_resource!(AudioResource, Audio);

#[derive(Serialize, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOptions {
    sources: Vec<String>,
    index: Option<usize>,
    cache_dir: Option<String>,
    auto_loop: Option<bool>,
}

fn handle_play_notify(id: u32, msg: AudioNotify) {
    run_on_event_loop(move || {
        let mut audio = PLAYING_MAP.with_borrow_mut(|m| m.get(&id).cloned());
        if let Some(a) = &mut audio {
            let target = a.clone();
            match msg {
                AudioNotify::Load(meta) => {
                    let mut event = Event::new("load", meta, target);
                    a.event_registration.emit_event(&mut event);
                }
                AudioNotify::TimeUpdate(time) => {
                    let mut event = Event::new("timeupdate", time, target);
                    a.event_registration.emit_event(&mut event);
                }
                AudioNotify::End => {
                    let mut event = Event::new("end", (), target);
                    a.event_registration.emit_event(&mut event);
                }
                AudioNotify::Finish => {
                    let mut event = Event::new("finish", (), target);
                    a.event_registration.emit_event(&mut event);
                    unregistry_playing(a);
                }
                AudioNotify::Pause => {
                    let mut event = Event::new("pause", (), target);
                    a.event_registration.emit_event(&mut event);
                }
                AudioNotify::Stop => {
                    let mut event = Event::new("stop", (), target);
                    a.event_registration.emit_event(&mut event);
                }
                AudioNotify::CurrentChange(info) => {
                    let mut event = Event::new("currentchange", info, target);
                    a.event_registration.emit_event(&mut event);
                }
            }
        }
    });
}

fn registry_playing(audio: &AudioResource) {
    let audio = audio.clone();
    PLAYING_MAP.with_borrow_mut(move |m| {
        m.insert(audio.id, audio);
    })
}

fn unregistry_playing(audio: &AudioResource) {
    let id = audio.id;
    PLAYING_MAP.with_borrow_mut(move |m| {
        m.remove(&id);
    })
}

pub fn audio_create(options: AudioOptions) -> Result<AudioResource, Error> {
    let id = NEXT_ID.get();
    NEXT_ID.set(id + 1);

    let sources = AudioSources {
        urls: options.sources,
        next_index: options.index.unwrap_or(0),
        cache_dir: options.cache_dir,
        auto_loop: options.auto_loop.unwrap_or(false),
        download_handle: None,
    };
    let audio = Audio::new(id, Arc::new(Mutex::new(sources)));
    Ok(AudioResource::new(audio))
}

pub fn audio_play(audio: AudioResource) -> Result<(), Error> {
    registry_playing(&audio);
    PLAYER.with(move |p| {
        p.play(audio.id, audio.sources.clone())
    });
    Ok(())
}

pub fn audio_pause(audio: AudioResource) -> Result<(), Error> {
    PLAYER.with(|p| {
        p.pause(audio.id)
    });
    Ok(())
}

pub fn audio_stop(audio: AudioResource) -> Result<(), Error> {
    unregistry_playing(&audio);
    PLAYER.with(|p| {
        p.stop(audio.id)
    });
    Ok(())
}

pub fn audio_add_event_listener(mut audio: AudioResource, event_type: String, callback: JsValue) -> Result<i32, Error> {
    let er = &mut audio.event_registration;
    Ok(er.add_js_event_listener(&event_type, callback))
}

pub fn audio_remove_event_listener(mut audio: AudioResource, event_type: String, id: u32) -> Result<(), Error> {
    audio.event_registration.remove_event_listener(&event_type, id);
    Ok(())
}