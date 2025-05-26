#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
pub mod player;

use player::{Player, Track, PlayerEvent};
use qmetaobject::{qt_base_class, qt_method, qt_property, qt_signal, QObject, QPointer, QmlEngine};
use std::ffi::CStr;

fn main() {
    // Import the function from the correct module
    use qmetaobject::qml_register_type;
    let qml_name = CStr::from_bytes_with_nul(b"MusicPlayer\0").unwrap();
    let qml_elem = CStr::from_bytes_with_nul(b"MusicPlayer\0").unwrap();
    qml_register_type::<MusicPlayer>(qml_name, 1, 0, qml_elem);
    let mut engine = QmlEngine::new();
    // engine.load_file("main.qml".into());
    engine.load_data(qmetaobject::QByteArray::from(include_bytes!("../main.qml").as_ref()));
    println!("Hi");
    engine.exec();
    println!("Bye");
}

lazy_static::lazy_static! {
    static ref THREAD_POOL: rayon::ThreadPool = rayon::ThreadPoolBuilder::new().build().unwrap();
}

pub fn qt_queued_callback<T: QObject + 'static, T2: Send, F: FnMut(&T, T2) + 'static>(qobj: &T, mut cb: F) -> impl Fn(T2) + Send + Sync + Clone {
    let qptr = QPointer::from(qobj);
    qmetaobject::queued_callback(move |arg| {
        if let Some(this) = qptr.as_pinned() {
            let this = this.borrow();
            cb(this, arg);
        }
    })
}pub fn qt_queued_callback_mut<T: QObject + 'static, T2: Send, F: FnMut(&mut T, T2) + 'static>(qptr: QPointer<T>, mut cb: F) -> impl Fn(T2) + Send + Sync + Clone {
    qmetaobject::queued_callback(move |arg| {
        if let Some(this) = qptr.as_pinned() {
            let mut this = this.borrow_mut();
            cb(&mut this, arg);
        }
    })
}

pub fn run_threaded<F>(cb: F) where F: FnOnce() + Send + 'static {
    THREAD_POOL.spawn(cb);
}

#[derive(QObject, Default)]
pub struct MusicPlayer {
    base: qt_base_class!(trait QObject),
    file_path: qt_property!(String; NOTIFY file_path_changed),
    file_path_changed: qt_signal!(),
    volume: qt_property!(f32; NOTIFY volume_changed),
    volume_changed: qt_signal!(),
    muted: qt_property!(bool; NOTIFY muted_changed),
    muted_changed: qt_signal!(),
    set_file: qt_method!(fn(&mut self, path: String)),
    set_volume: qt_method!(fn(&mut self, v: f32)),
    set_muted: qt_method!(fn(&mut self, m: bool)),
    play: qt_method!(fn(&mut self)),
    pause: qt_method!(fn(&self)),
    seek: qt_method!(fn(&mut self, pos: f32)),
    sync_progress: qt_signal!(progress: f64),
    player: Player
}

impl MusicPlayer {
    fn set_file(&mut self, path: String) {
        self.file_path = path.clone();
        self.file_path_changed();
    }
    fn set_volume(&mut self, v: f32) {
        self.volume = v;
        self.volume_changed();
        self.player.set_volume(v);
    }
    fn set_muted(&mut self, m: bool) {
        self.muted = m;
        self.muted_changed();
        self.player.set_muted(m);
    }
    fn play(&mut self) {
        self.player.add_track(Track {
            file_path: self.file_path.clone()
        });
        self.player.play();
        self.test();
    }
    fn test(&self) {
        let recv = self.player.out_evt_receiver();
        let qptr = QPointer::<MusicPlayer>::from(self);
        let cb = qt_queued_callback_mut(qptr.clone(), |this, val| {
            this.sync_progress(val);
        });
        let play = qt_queued_callback_mut(qptr, |this, _: ()| {
            this.player.play();
        });
        run_threaded(move || {
            let recv = recv.lock();
            println!("Starting progress receiver thread");
            if recv.is_err() {
                println!("Failed to lock receiver");
                return;
            }
            if let Ok(recv) = recv {
                while let Ok(event) = recv.recv() {
                    match event {
                        PlayerEvent::Progress(p) => {
                            cb(p);
                        },
                        PlayerEvent::End => {
                            println!("Playback ended");
                            play(());
                        },
                    }
                }
            }
        });
        }
    fn pause(&mut self) {
        self.player.pause();
    }
    fn seek(&mut self, pos: f32) {
        self.player.seek(pos);
    }
}
