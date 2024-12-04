use crate as lento;
use serde::{Deserialize, Serialize};
use winit::keyboard::{ModifiersState, NamedKey};
use lento_macros::event;
use crate::base::{CaretDetail, MouseDetail, Rect, ScrollEventDetail, TextChangeDetail, TextUpdateDetail, TouchDetail};
use crate::{base, define_event};

pub const KEY_MOD_CTRL: u32 = 0x1;
pub const KEY_MOD_ALT: u32 = 0x1 << 1;
pub const KEY_MOD_META: u32 = 0x1 << 2;
pub const KEY_MOD_SHIFT: u32 = 0x1 << 3;


#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyEventDetail {
    pub modifiers: u32,
    pub ctrl_key: bool,
    pub alt_key: bool,
    pub meta_key: bool,
    pub shift_key: bool,
    #[serde(skip)]
    pub named_key: Option<NamedKey>,
    pub key: Option<String>,
    pub key_str: Option<String>,
    pub repeat: bool,
    pub pressed: bool,
}

pub fn build_modifier(state: &ModifiersState) -> u32 {
    let mut modifiers = 0;
    if state.alt_key() {
        modifiers |= KEY_MOD_ALT;
    }
    if state.control_key() {
        modifiers |= KEY_MOD_CTRL;
    }
    if state.super_key() {
        modifiers |= KEY_MOD_META;
    }
    if state.shift_key() {
        modifiers |= KEY_MOD_SHIFT;
    }
    modifiers
}



pub fn named_key_to_str(key: &NamedKey) -> &'static str {
    macro_rules! named_key_map {
        ($key: expr => $($name: ident,)*) => {
            match $key {
                $(
                    NamedKey::$name => stringify!($name),
                )*
                _ => "Unknown",
            }
        };
    }
    named_key_map!(
        key =>
        Alt,
        AltGraph,
        CapsLock,
        Control,
        Fn,
        FnLock,
        NumLock,
        ScrollLock,
        Shift,
        Symbol,
        SymbolLock,
        Meta,
        Hyper,
        Super,
        Enter,
        Tab,
        Space,
        ArrowDown,
        ArrowLeft,
        ArrowRight,
        ArrowUp,
        End,
        Home,
        PageDown,
        PageUp,
        Backspace,
        Clear,
        Copy,
        CrSel,
        Cut,
        Delete,
        EraseEof,
        ExSel,
        Insert,
        Paste,
        Redo,
        Undo,
        Accept,
        Again,
        Attn,
        Cancel,
        ContextMenu,
        Escape,
        Execute,
        Find,
        Help,
        Pause,
        Play,
        Props,
        Select,
        ZoomIn,
        ZoomOut,
        BrightnessDown,
        BrightnessUp,
        Eject,
        LogOff,
        Power,
        PowerOff,
        PrintScreen,
        Hibernate,
        Standby,
        WakeUp,
        AllCandidates,
        Alphanumeric,
        CodeInput,
        Compose,
        Convert,
        FinalMode,
        GroupFirst,
        GroupLast,
        GroupNext,
        GroupPrevious,
        ModeChange,
        NextCandidate,
        NonConvert,
        PreviousCandidate,
        Process,
        SingleCandidate,
        HangulMode,
        HanjaMode,
        JunjaMode,
        Eisu,
        Hankaku,
        Hiragana,
        HiraganaKatakana,
        KanaMode,
        KanjiMode,
        Katakana,
        Romaji,
        Zenkaku,
        ZenkakuHankaku,
        Soft1,
        Soft2,
        Soft3,
        Soft4,
        ChannelDown,
        ChannelUp,
        Close,
        MailForward,
        MailReply,
        MailSend,
        MediaClose,
        MediaFastForward,
        MediaPause,
        MediaPlay,
        MediaPlayPause,
        MediaRecord,
        MediaRewind,
        MediaStop,
        MediaTrackNext,
        MediaTrackPrevious,
        New,
        Open,
        Print,
        Save,
        SpellCheck,
        Key11,
        Key12,
        AudioBalanceLeft,
        AudioBalanceRight,
        AudioBassBoostDown,
        AudioBassBoostToggle,
        AudioBassBoostUp,
        AudioFaderFront,
        AudioFaderRear,
        AudioSurroundModeNext,
        AudioTrebleDown,
        AudioTrebleUp,
        AudioVolumeDown,
        AudioVolumeUp,
        AudioVolumeMute,
        MicrophoneToggle,
        MicrophoneVolumeDown,
        MicrophoneVolumeUp,
        MicrophoneVolumeMute,
        SpeechCorrectionList,
        SpeechInputToggle,
        LaunchApplication1,
        LaunchApplication2,
        LaunchCalendar,
        LaunchContacts,
        LaunchMail,
        LaunchMediaPlayer,
        LaunchMusicPlayer,
        LaunchPhone,
        LaunchScreenSaver,
        LaunchSpreadsheet,
        LaunchWebBrowser,
        LaunchWebCam,
        LaunchWordProcessor,
        BrowserBack,
        BrowserFavorites,
        BrowserForward,
        BrowserHome,
        BrowserRefresh,
        BrowserSearch,
        BrowserStop,
        AppSwitch,
        Call,
        Camera,
        CameraFocus,
        EndCall,
        GoBack,
        GoHome,
        HeadsetHook,
        LastNumberRedial,
        Notification,
        MannerMode,
        VoiceDial,
        TV,
        TV3DMode,
        TVAntennaCable,
        TVAudioDescription,
        TVAudioDescriptionMixDown,
        TVAudioDescriptionMixUp,
        TVContentsMenu,
        TVDataService,
        TVInput,
        TVInputComponent1,
        TVInputComponent2,
        TVInputComposite1,
        TVInputComposite2,
        TVInputHDMI1,
        TVInputHDMI2,
        TVInputHDMI3,
        TVInputHDMI4,
        TVInputVGA1,
        TVMediaContext,
        TVNetwork,
        TVNumberEntry,
        TVPower,
        TVRadioService,
        TVSatellite,
        TVSatelliteBS,
        TVSatelliteCS,
        TVSatelliteToggle,
        TVTerrestrialAnalog,
        TVTerrestrialDigital,
        TVTimer,
        AVRInput,
        AVRPower,
        ColorF0Red,
        ColorF1Green,
        ColorF2Yellow,
        ColorF3Blue,
        ColorF4Grey,
        ColorF5Brown,
        ClosedCaptionToggle,
        Dimmer,
        DisplaySwap,
        DVR,
        Exit,
        FavoriteClear0,
        FavoriteClear1,
        FavoriteClear2,
        FavoriteClear3,
        FavoriteRecall0,
        FavoriteRecall1,
        FavoriteRecall2,
        FavoriteRecall3,
        FavoriteStore0,
        FavoriteStore1,
        FavoriteStore2,
        FavoriteStore3,
        Guide,
        GuideNextDay,
        GuidePreviousDay,
        Info,
        InstantReplay,
        Link,
        ListProgram,
        LiveContent,
        Lock,
        MediaApps,
        MediaAudioTrack,
        MediaLast,
        MediaSkipBackward,
        MediaSkipForward,
        MediaStepBackward,
        MediaStepForward,
        MediaTopMenu,
        NavigateIn,
        NavigateNext,
        NavigateOut,
        NavigatePrevious,
        NextFavoriteChannel,
        NextUserProfile,
        OnDemand,
        Pairing,
        PinPDown,
        PinPMove,
        PinPToggle,
        PinPUp,
        PlaySpeedDown,
        PlaySpeedReset,
        PlaySpeedUp,
        RandomToggle,
        RcLowBattery,
        RecordSpeedNext,
        RfBypass,
        ScanChannelsToggle,
        ScreenModeNext,
        Settings,
        SplitScreenToggle,
        STBInput,
        STBPower,
        Subtitle,
        Teletext,
        VideoModeNext,
        Wink,
        ZoomToggle,
        F1,
        F2,
        F3,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        F10,
        F11,
        F12,
        F13,
        F14,
        F15,
        F16,
        F17,
        F18,
        F19,
        F20,
        F21,
        F22,
        F23,
        F24,
        F25,
        F26,
        F27,
        F28,
        F29,
        F30,
        F31,
        F32,
        F33,
        F34,
        F35,
    )
}

#[event]
pub struct ClickEvent(pub MouseDetail);

#[event]
pub struct MouseUpEvent(pub MouseDetail);

#[event]
pub struct MouseDownEvent(pub MouseDetail);

#[event]
pub struct MouseMoveEvent(pub MouseDetail);

#[event]
pub struct MouseEnterEvent(pub MouseDetail);

#[event]
pub struct MouseLeaveEvent(pub MouseDetail);

#[event]
pub struct KeyDownEvent(pub KeyEventDetail);

#[event]
pub struct KeyUpEvent(pub KeyEventDetail);

#[event]
pub struct MouseWheelEvent {
    pub cols: f32,
    pub rows: f32,
}

#[event]
pub struct TextUpdateEvent {
    pub value: String,
}

#[event]
pub struct TouchStartEvent(pub TouchDetail);

#[event]
pub struct TouchMoveEvent(pub TouchDetail);

#[event]
pub struct TouchEndEvent(pub TouchDetail);

#[event]
pub struct TouchCancelEvent(pub TouchDetail);

#[event]
pub struct FocusEvent;

#[event]
pub struct BlurEvent;

#[event]
pub struct FocusShiftEvent;

#[event]
pub struct TextChangeEvent {
    pub value: String,
}

#[event]
pub struct ScrollEvent {
    pub scroll_top: f32,
    pub scroll_left: f32,
}

#[event]
pub struct DragStartEvent;

#[event]
pub struct DragOverEvent;

#[event]
pub struct DropEvent;

#[event]
pub struct BoundsChangeEvent {
    pub origin_bounds: base::Rect,
}

#[event]
pub struct CaretChangeEvent {
    pub position: usize,
    pub origin_bounds: Rect,
    pub bounds: Rect,
}
