use serde::{Deserialize, Serialize};
use winit::keyboard::{ModifiersState, NamedKey};
use crate::base::{CaretDetail, MouseDetail, TextUpdateDetail, TouchDetail};
use crate::define_event;

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DragStartEventDetail {

}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DragOverEventDetail {

}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DropEventDetail {

}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MouseWheelDetail {
    pub cols: f32,
    pub rows: f32,
}

define_event!(CaretEvent,       CaretEventBind,       "caretchange", bind_caret_change, emit_caret_change, AcceptCaretEvent,       accept_caret_change, CaretDetail);
define_event!(MouseDownEvent,   MouseDownEventBind,   "mousedown",   bind_mouse_down,   emit_mouse_down,   AcceptMouseDownEvent,   accept_mouse_down,   MouseDetail);
define_event!(MouseUpEvent,     MouseUpEventBind,     "mouseup",     bind_mouse_up,     emit_mouse_up,     AcceptMouseUpEvent,     accept_mouse_up,     MouseDetail);
define_event!(MouseClickEvent,  ClickEventBind,       "click",       bind_click,        emit_click,        AcceptClickEvent,       accept_click,        MouseDetail);
define_event!(MouseMoveEvent,   MouseMoveEventBind,   "mousemove",   bind_mouse_move,   emit_mouse_move,   AcceptMouseMoveEvent,   accept_mouse_move,   MouseDetail);
define_event!(MouseEnterEvent,  MouseEnterEventBind,  "mouseenter",  bind_mouse_enter,  emit_mouse_enter,  AcceptMouseEnterEvent,  accept_mouse_enter,  MouseDetail);
define_event!(MouseLeaveEvent,  MouseLeaveEventBind,  "mouseleave",  bind_mouse_leave,  emit_mouse_leave,  AcceptMouseLeaveEvent,  accept_mouse_leave,  MouseDetail);
define_event!(KeyDownEvent,     KeyDownEventBind,     "keydown",     bind_key_down,     emit_key_down,     AcceptKeyDownEvent,     accept_key_down,     KeyEventDetail);
define_event!(KeyUpEvent,       KeyUpEventBind,       "keyup",       bind_key_up,       emit_key_up,       AcceptKeyUpEvent,       accept_key_up,       KeyEventDetail);
define_event!(MouseWheelEvent,  MouseWheelEventBind,  "mousewheel",  bind_mouse_wheel,  emit_mouse_wheel,  AcceptMouseWheelEvent,  accept_mouse_wheel,  MouseWheelDetail);
define_event!(TextUpdateEvent,  TextUpdateEventBind,  "textupdate",  bind_text_update,  emit_text_update,  AcceptTextUpdateEvent,  accept_text_update,  TextUpdateDetail);
define_event!(TouchStartEvent,  TouchStartEventBind,  "touchstart",  bind_touch_start,  emit_touch_start,  AcceptTouchStartEvent,  accept_touch_start,  TouchDetail);
define_event!(TouchMoveEvent,   TouchMoveEventBind,   "touchmove",   bind_touch_move,   emit_touch_move,   AcceptTouchMoveEvent,   accept_touch_move,   TouchDetail);
define_event!(TouchEndEvent,    TouchEndEventBind,    "touchend",    bind_touch_end,    emit_touch_end,    AcceptTouchEndEvent,    accept_touch_end,    TouchDetail);
define_event!(TouchCancelEvent, TouchCancelEventBind, "touchcancel", bind_touch_cancel, emit_touch_cancel, AcceptTouchCancelEvent, accept_touch_cancel, TouchDetail);
define_event!(FocusEvent,       FocusEventBind,       "focus",       bind_focus,        emit_focus,        AcceptFocusEvent,       accept_focus,        ());
// event_api!(Scroll, "scroll", bind_scroll, ScrollEventDetail);

