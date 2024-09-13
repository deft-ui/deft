use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Write};
use std::sync::{Arc, mpsc, Mutex};
use std::sync::mpsc::Sender;
use std::{fs, thread};
use std::thread::JoinHandle;
use std::time::Duration;
use anyhow::Error;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use crate::data_dir::get_data_path;
use crate::ext::audio_player::AudioNotify::{End, Finish, Load, TimeUpdate};

#[derive(Serialize, Deserialize, Debug)]
pub struct AudioCurrentChangeInfo {
    index: usize,
}

#[derive(Serialize, Deserialize)]
pub struct AudioMeta {
    index: usize,
    duration: f32,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateTimeInfo {
    index: usize,
    time: f32,
}

#[derive(Serialize, Deserialize)]
pub struct EndInfo {
    index: usize,
}

pub struct AudioSources {
    pub urls: Vec<String>,
    pub next_index: usize,
    pub cache_dir: Option<String>,
    pub auto_loop: bool,
    pub download_handle: Option<JoinHandle<(usize, String)>>,
}

enum SourceResult {
    Some((usize, Decoder<BufReader<File>>)),
    None,
    Pending,
}

impl AudioSources {
    pub fn next_source(&mut self) -> SourceResult {
        if self.urls.is_empty() {
            return SourceResult::None;
        }
        if let Some(dh) = self.download_handle.take() {
            if !dh.is_finished() {
                self.download_handle = Some(dh);
                return SourceResult::Pending;
            }
            return match dh.join() {
                Ok((id, path)) => {
                    match Self::create_source(path.as_str()) {
                        Ok(src) => SourceResult::Some((id, src)),
                        Err(err) => {
                            eprintln!("load source failed:{}", err);
                            self.next_source()
                        }
                    }
                }
                Err(e) => {
                    println!("error: {:?}", e);
                    self.next_source()
                }
            };
        }
        let idx = self.next_index;
        if let Some(s) = self.urls.get(idx) {
            self.next_index += 1;
            if s.starts_with("http://") || s.starts_with("https://") {
                let s = s.to_string();
                self.download_handle = Some(thread::spawn(move || {
                    let cache_dir = get_data_path("music-cache");
                    if !cache_dir.exists() {
                        fs::create_dir_all(&cache_dir).unwrap();
                    }
                    let key = base16ct::lower::encode_string(&Sha1::digest(&s));
                    let path = cache_dir.join(key);
                    if !path.exists() {
                        println!("downloading {} to {:?}", &s, &path);

                        let rsp = reqwest::blocking::get(&s).unwrap();
                        let bytes = rsp.bytes().unwrap();
                        //TODO optimize memory
                        let vec = bytes.to_vec();
                        let mut file = File::create_new(&path).unwrap();
                        file.write_all(&vec).unwrap();
                    }
                    (idx, path.to_string_lossy().to_string())
                }));
                return SourceResult::Pending;
            }

            return if let Ok(s) = Self::create_source(s) {
                SourceResult::Some((idx, s))
            } else {
                self.next_source()
            };
        } else {
            if self.auto_loop {
                self.next_index = 0;
                self.next_source()
            } else {
                SourceResult::None
            }
        }
    }

    fn create_source(path: &str) -> Result<Decoder<BufReader<File>>, Error> {
        let file = File::open(path)?;
        let source = Decoder::new(BufReader::new(file))?;
        Ok(source)
    }
}

pub enum AudioNotify {
    CurrentChange(AudioCurrentChangeInfo),
    Load(AudioMeta),
    TimeUpdate(f32),
    Pause,
    Stop,
    End,
    Finish,
}

pub struct AudioServer {
    sender: Sender<AudioReq>,
}

struct AudioStream {
    stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
}

struct AudioPlayContext {
    audio_stream: Option<AudioStream>,
    last_pos: f32,
    playing: bool,
    source: Arc<Mutex<AudioSources>>,
}

impl AudioPlayContext {
    pub fn current_index(&self) -> usize {
        let sources = self.source.lock().unwrap();
        if sources.next_index == 0 {
            sources.urls.len() - 1
        } else {
            sources.next_index - 1
        }
    }
}

enum AudioReq {
    Play(u32, Arc<Mutex<AudioSources>>),
    Pause(u32),
    Stop(u32),
}

impl AudioServer {
    pub fn new<F: FnMut(u32, AudioNotify) + Send + 'static>(mut notify_handler: F) -> Self {
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut playing_list: HashMap<u32, AudioPlayContext> = HashMap::new();
            fn play_next_source<F: FnMut(u32, AudioNotify) + Send + 'static>(id: u32, audio_context: &mut AudioPlayContext, notify_handler: &mut F) -> bool {
                let mut sources = audio_context.source.lock().unwrap();
                let source = sources.next_source();
                match source {
                    SourceResult::Some((index, source)) => {
                        let mut duration = 0f32;
                        if let Some(d) = source.total_duration() {
                            duration = d.as_secs_f32();
                        }
                        let (stream, handle) = OutputStream::try_default().unwrap();
                        let sink = Sink::try_new(&handle).unwrap();
                        sink.append(source);
                        audio_context.audio_stream = Some(AudioStream {
                            stream,
                            handle,
                            sink,
                        });
                        audio_context.playing = true;
                        let meta = AudioMeta {
                            index,
                            duration,
                        };
                        notify_handler(id, Load(meta));
                        true
                    }
                    SourceResult::None => false,
                    SourceResult::Pending => true,
                }
            }
            loop {
                if let Ok(msg) = receiver.recv_timeout(Duration::from_millis(100)) {
                    match msg {
                        AudioReq::Play(id, source) => {
                            if let Some(s) = playing_list.get_mut(&id) {
                                if let Some(a) = &mut s.audio_stream {
                                    s.playing = true;
                                    a.sink.play();
                                }
                            } else {
                                let mut audio_context = AudioPlayContext {
                                    audio_stream: None,
                                    last_pos: 0.0,
                                    playing: false,
                                    source,
                                };
                                if play_next_source(id, &mut audio_context, &mut notify_handler) {
                                    playing_list.insert(id, audio_context);
                                }
                            }
                        }
                        AudioReq::Pause(id) => {
                            if let Some(s) = playing_list.get_mut(&id) {
                                s.playing = false;
                                if let Some(s) = &mut s.audio_stream {
                                    s.sink.pause();
                                    notify_handler(id, AudioNotify::Pause);
                                }
                            }
                        }
                        AudioReq::Stop(id) => {
                            if let Some(s) = playing_list.get_mut(&id) {
                                s.playing = false;
                                if let Some(s) = &mut s.audio_stream {
                                    s.sink.stop();
                                }
                                playing_list.remove(&id);
                                notify_handler(id, AudioNotify::Stop);
                            }
                        }
                    }
                } else {
                    playing_list.retain(|k, v| {
                        let id = *k;
                        let is_end_pending = v.audio_stream.as_ref()
                            .map(|it| it.sink.empty())
                            .unwrap_or(false);
                        if is_end_pending {
                            v.audio_stream = None;
                            let id = *k;
                            notify_handler(id, End);
                        }
                        return if let Some(a) = &mut v.audio_stream {
                            let pos = a.sink.get_pos().as_secs_f32();
                            if v.playing && pos != v.last_pos {
                                v.last_pos = pos;
                                notify_handler(id, TimeUpdate(pos));
                            }
                            true
                        } else {
                            let next = play_next_source(*k, v, &mut notify_handler);
                            if !next {
                                notify_handler(id, Finish);
                            } else if is_end_pending {
                                notify_handler(id, AudioNotify::CurrentChange(AudioCurrentChangeInfo {
                                    index: v.current_index()
                                }));
                            }
                            next
                        };
                    })
                }
            }
        });
        Self {
            sender,
        }
    }

    pub fn play(&self, id: u32, source: Arc<Mutex<AudioSources>>) {
        self.sender.send(AudioReq::Play(id, source)).unwrap();
    }

    pub fn pause(&self, id: u32) {
        self.sender.send(AudioReq::Pause(id)).unwrap()
    }

    pub fn stop(&self, id: u32) {
        self.sender.send(AudioReq::Stop(id)).unwrap();
    }
}