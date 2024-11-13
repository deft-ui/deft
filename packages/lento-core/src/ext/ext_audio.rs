use crate as lento;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use crate::base::{Event, EventRegistration};
use crate::event_loop::run_on_event_loop;
use crate::ext::audio_player::{AudioNotify, AudioServer, AudioSources};
use crate::{define_resource, js_deserialize, js_value};
use anyhow::{anyhow, Error};
use lento_macros::{js_methods, mrc_object};
use quick_js::JsValue;
use serde::{Deserialize, Serialize};

thread_local! {
    pub static NEXT_ID: Cell<u32> = Cell::new(1);
    pub static PLAYING_MAP: RefCell<HashMap<u32, Audio >> = RefCell::new(HashMap::new());
    pub static PLAYER: AudioServer = AudioServer::new(handle_play_notify);
}

#[mrc_object]
pub struct Audio {
    id: u32,
    event_registration: EventRegistration<Audio>,
    sources: Arc<Mutex<AudioSources>>,
}

impl AudioData {
    pub fn new(id: u32, sources: Arc<Mutex<AudioSources>>) -> Self {
        Self {
            id,
            event_registration: EventRegistration::new(),
            sources,
        }
    }
}


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

fn registry_playing(audio: &Audio) {
    let audio = audio.clone();
    PLAYING_MAP.with_borrow_mut(move |m| {
        m.insert(audio.id, audio);
    })
}

fn unregistry_playing(audio: &Audio) {
    let id = audio.id;
    PLAYING_MAP.with_borrow_mut(move |m| {
        m.remove(&id);
    })
}

define_resource!(Audio);
js_value!(Audio);
js_deserialize!(AudioOptions);


#[js_methods]
impl Audio {

    #[js_func]
    pub fn create(options: AudioOptions) -> Result<Audio, Error> {
        let id = NEXT_ID.get();
        NEXT_ID.set(id + 1);

        let sources = AudioSources {
            urls: options.sources,
            next_index: options.index.unwrap_or(0),
            cache_dir: options.cache_dir,
            auto_loop: options.auto_loop.unwrap_or(false),
            download_handle: None,
        };
        let audio = AudioData::new(id, Arc::new(Mutex::new(sources)));
        Ok(audio.to_ref())
    }

    #[js_func]
    pub fn play(audio: Audio) -> Result<(), Error> {
        registry_playing(&audio);
        PLAYER.with(move |p| {
            p.play(audio.id, audio.sources.clone())
        });
        Ok(())
    }

    #[js_func]
    pub fn pause(audio: Audio) -> Result<(), Error> {
        PLAYER.with(|p| {
            p.pause(audio.id)
        });
        Ok(())
    }

    #[js_func]
    pub fn stop(&self) -> Result<(), Error> {
        unregistry_playing(&self);
        PLAYER.with(|p| {
            p.stop(self.id)
        });
        Ok(())
    }

    #[js_func]
    pub fn add_event_listener(&mut self, event_type: String, callback: JsValue) -> Result<i32, Error> {
        let er = &mut self.event_registration;
        Ok(er.add_js_event_listener(&event_type, callback))
    }

    #[js_func]
    pub fn remove_event_listener(&mut self, event_type: String, id: u32) -> Result<(), Error> {
        self.event_registration.remove_event_listener(&event_type, id);
        Ok(())
    }
}