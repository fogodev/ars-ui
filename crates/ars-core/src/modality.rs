//! Shared input-modality state and normalization primitives.
//!
//! This module provides the platform-agnostic modality contract shared by
//! interaction, accessibility, and adapter layers. It intentionally models
//! modality as instance-scoped state so each provider root, window, or scene
//! can track its own last input modality independently.

use core::sync::atomic::{AtomicBool, AtomicU8, Ordering};

/// The input modality that initiated an interaction.
///
/// Matches the values exposed by the Pointer Events API, extended with
/// keyboard and virtual activation for accessibility and scripted focus flows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerType {
    /// Physical mouse or trackpad.
    Mouse,
    /// Finger on a touchscreen.
    Touch,
    /// Stylus or digital pen.
    Pen,
    /// Keyboard-driven interaction.
    Keyboard,
    /// Programmatic or virtual-cursor activation.
    Virtual,
}

/// Raw platform keyboard modifier state captured from an input event.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct KeyModifiers {
    /// Whether the Shift key was held.
    pub shift: bool,
    /// Whether the physical Ctrl key was held.
    pub ctrl: bool,
    /// Whether the Alt/Option key was held.
    pub alt: bool,
    /// Whether the Meta/Cmd/Windows key was held.
    pub meta: bool,
}

/// Named keyboard keys normalized from W3C `KeyboardEvent.key` values.
///
/// Reference: [W3C UI Events KeyboardEvent key Values](https://www.w3.org/TR/uievents-key/).
///
/// Printable characters are intentionally excluded. Consumers should use a
/// separate character field for text input and reserve this enum for named keys.
#[expect(
    missing_docs,
    reason = "The variants mirror the W3C named-key registry and are self-describing."
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum KeyboardKey {
    Unidentified,
    Alt,
    AltGraph,
    CapsLock,
    Control,
    Fn,
    FnLock,
    Meta,
    NumLock,
    ScrollLock,
    Shift,
    Symbol,
    SymbolLock,
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
    Dead,
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
    Tv,
    Tv3DMode,
    TvAntennaCable,
    TvAudioDescription,
    TvAudioDescriptionMixDown,
    TvAudioDescriptionMixUp,
    TvContentsMenu,
    TvDataService,
    TvInput,
    TvInputComponent1,
    TvInputComponent2,
    TvInputComposite1,
    TvInputComposite2,
    TvInputHdmi1,
    TvInputHdmi2,
    TvInputHdmi3,
    TvInputHdmi4,
    TvInputVga1,
    TvMediaContext,
    TvNetwork,
    TvNumberEntry,
    TvPower,
    TvRadioService,
    TvSatellite,
    TvSatelliteBS,
    TvSatelliteCS,
    TvSatelliteToggle,
    TvTerrestrialAnalog,
    TvTerrestrialDigital,
    TvTimer,
    AvrInput,
    AvrPower,
    ColorF0Red,
    ColorF1Green,
    ColorF2Yellow,
    ColorF3Blue,
    ColorF4Grey,
    ColorF5Brown,
    ClosedCaptionToggle,
    Dimmer,
    DisplaySwap,
    Dvr,
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
    StbInput,
    StbPower,
    Subtitle,
    Teletext,
    VideoModeNext,
    Wink,
    ZoomToggle,
}

impl KeyboardKey {
    /// Parses a W3C key string into a named key.
    ///
    /// Unrecognized values, including printable character keys, map to
    /// [`KeyboardKey::Unidentified`].
    #[must_use]
    pub fn from_key_str(value: &str) -> Self {
        match value {
            " " | "Space" => Self::Space,
            "11" => Self::Key11,
            "12" => Self::Key12,
            "TV" => Self::Tv,
            "TV3DMode" => Self::Tv3DMode,
            "TVAntennaCable" => Self::TvAntennaCable,
            "TVAudioDescription" => Self::TvAudioDescription,
            "TVAudioDescriptionMixDown" => Self::TvAudioDescriptionMixDown,
            "TVAudioDescriptionMixUp" => Self::TvAudioDescriptionMixUp,
            "TVContentsMenu" => Self::TvContentsMenu,
            "TVDataService" => Self::TvDataService,
            "TVInput" => Self::TvInput,
            "TVInputComponent1" => Self::TvInputComponent1,
            "TVInputComponent2" => Self::TvInputComponent2,
            "TVInputComposite1" => Self::TvInputComposite1,
            "TVInputComposite2" => Self::TvInputComposite2,
            "TVInputHDMI1" => Self::TvInputHdmi1,
            "TVInputHDMI2" => Self::TvInputHdmi2,
            "TVInputHDMI3" => Self::TvInputHdmi3,
            "TVInputHDMI4" => Self::TvInputHdmi4,
            "TVInputVGA1" => Self::TvInputVga1,
            "TVMediaContext" => Self::TvMediaContext,
            "TVNetwork" => Self::TvNetwork,
            "TVNumberEntry" => Self::TvNumberEntry,
            "TVPower" => Self::TvPower,
            "TVRadioService" => Self::TvRadioService,
            "TVSatellite" => Self::TvSatellite,
            "TVSatelliteBS" => Self::TvSatelliteBS,
            "TVSatelliteCS" => Self::TvSatelliteCS,
            "TVSatelliteToggle" => Self::TvSatelliteToggle,
            "TVTerrestrialAnalog" => Self::TvTerrestrialAnalog,
            "TVTerrestrialDigital" => Self::TvTerrestrialDigital,
            "TVTimer" => Self::TvTimer,
            "AVRInput" => Self::AvrInput,
            "AVRPower" => Self::AvrPower,
            "DVR" => Self::Dvr,
            "STBInput" => Self::StbInput,
            "STBPower" => Self::StbPower,
            _ => match value {
                "Alt" => Self::Alt,
                "AltGraph" => Self::AltGraph,
                "CapsLock" => Self::CapsLock,
                "Control" => Self::Control,
                "Fn" => Self::Fn,
                "FnLock" => Self::FnLock,
                "Meta" => Self::Meta,
                "NumLock" => Self::NumLock,
                "ScrollLock" => Self::ScrollLock,
                "Shift" => Self::Shift,
                "Symbol" => Self::Symbol,
                "SymbolLock" => Self::SymbolLock,
                "Hyper" => Self::Hyper,
                "Super" => Self::Super,
                "Enter" => Self::Enter,
                "Tab" => Self::Tab,
                "ArrowDown" => Self::ArrowDown,
                "ArrowLeft" => Self::ArrowLeft,
                "ArrowRight" => Self::ArrowRight,
                "ArrowUp" => Self::ArrowUp,
                "End" => Self::End,
                "Home" => Self::Home,
                "PageDown" => Self::PageDown,
                "PageUp" => Self::PageUp,
                "Backspace" => Self::Backspace,
                "Clear" => Self::Clear,
                "Copy" => Self::Copy,
                "CrSel" => Self::CrSel,
                "Cut" => Self::Cut,
                "Delete" => Self::Delete,
                "EraseEof" => Self::EraseEof,
                "ExSel" => Self::ExSel,
                "Insert" => Self::Insert,
                "Paste" => Self::Paste,
                "Redo" => Self::Redo,
                "Undo" => Self::Undo,
                "Accept" => Self::Accept,
                "Again" => Self::Again,
                "Attn" => Self::Attn,
                "Cancel" => Self::Cancel,
                "ContextMenu" => Self::ContextMenu,
                "Escape" => Self::Escape,
                "Execute" => Self::Execute,
                "Find" => Self::Find,
                "Help" => Self::Help,
                "Pause" => Self::Pause,
                "Play" => Self::Play,
                "Props" => Self::Props,
                "Select" => Self::Select,
                "ZoomIn" => Self::ZoomIn,
                "ZoomOut" => Self::ZoomOut,
                "BrightnessDown" => Self::BrightnessDown,
                "BrightnessUp" => Self::BrightnessUp,
                "Eject" => Self::Eject,
                "LogOff" => Self::LogOff,
                "Power" => Self::Power,
                "PowerOff" => Self::PowerOff,
                "PrintScreen" => Self::PrintScreen,
                "Hibernate" => Self::Hibernate,
                "Standby" => Self::Standby,
                "WakeUp" => Self::WakeUp,
                "AllCandidates" => Self::AllCandidates,
                "Alphanumeric" => Self::Alphanumeric,
                "CodeInput" => Self::CodeInput,
                "Compose" => Self::Compose,
                "Convert" => Self::Convert,
                "Dead" => Self::Dead,
                "FinalMode" => Self::FinalMode,
                "GroupFirst" => Self::GroupFirst,
                "GroupLast" => Self::GroupLast,
                "GroupNext" => Self::GroupNext,
                "GroupPrevious" => Self::GroupPrevious,
                "ModeChange" => Self::ModeChange,
                "NextCandidate" => Self::NextCandidate,
                "NonConvert" => Self::NonConvert,
                "PreviousCandidate" => Self::PreviousCandidate,
                "Process" => Self::Process,
                "SingleCandidate" => Self::SingleCandidate,
                "HangulMode" => Self::HangulMode,
                "HanjaMode" => Self::HanjaMode,
                "JunjaMode" => Self::JunjaMode,
                "Eisu" => Self::Eisu,
                "Hankaku" => Self::Hankaku,
                "Hiragana" => Self::Hiragana,
                "HiraganaKatakana" => Self::HiraganaKatakana,
                "KanaMode" => Self::KanaMode,
                "KanjiMode" => Self::KanjiMode,
                "Katakana" => Self::Katakana,
                "Romaji" => Self::Romaji,
                "Zenkaku" => Self::Zenkaku,
                "ZenkakuHankaku" => Self::ZenkakuHankaku,
                "F1" => Self::F1,
                "F2" => Self::F2,
                "F3" => Self::F3,
                "F4" => Self::F4,
                "F5" => Self::F5,
                "F6" => Self::F6,
                "F7" => Self::F7,
                "F8" => Self::F8,
                "F9" => Self::F9,
                "F10" => Self::F10,
                "F11" => Self::F11,
                "F12" => Self::F12,
                "Soft1" => Self::Soft1,
                "Soft2" => Self::Soft2,
                "Soft3" => Self::Soft3,
                "Soft4" => Self::Soft4,
                "ChannelDown" => Self::ChannelDown,
                "ChannelUp" => Self::ChannelUp,
                "Close" => Self::Close,
                "MailForward" => Self::MailForward,
                "MailReply" => Self::MailReply,
                "MailSend" => Self::MailSend,
                "MediaClose" => Self::MediaClose,
                "MediaFastForward" => Self::MediaFastForward,
                "MediaPause" => Self::MediaPause,
                "MediaPlay" => Self::MediaPlay,
                "MediaPlayPause" => Self::MediaPlayPause,
                "MediaRecord" => Self::MediaRecord,
                "MediaRewind" => Self::MediaRewind,
                "MediaStop" => Self::MediaStop,
                "MediaTrackNext" => Self::MediaTrackNext,
                "MediaTrackPrevious" => Self::MediaTrackPrevious,
                "New" => Self::New,
                "Open" => Self::Open,
                "Print" => Self::Print,
                "Save" => Self::Save,
                "SpellCheck" => Self::SpellCheck,
                "AudioBalanceLeft" => Self::AudioBalanceLeft,
                "AudioBalanceRight" => Self::AudioBalanceRight,
                "AudioBassBoostDown" => Self::AudioBassBoostDown,
                "AudioBassBoostToggle" => Self::AudioBassBoostToggle,
                "AudioBassBoostUp" => Self::AudioBassBoostUp,
                "AudioFaderFront" => Self::AudioFaderFront,
                "AudioFaderRear" => Self::AudioFaderRear,
                "AudioSurroundModeNext" => Self::AudioSurroundModeNext,
                "AudioTrebleDown" => Self::AudioTrebleDown,
                "AudioTrebleUp" => Self::AudioTrebleUp,
                "AudioVolumeDown" => Self::AudioVolumeDown,
                "AudioVolumeUp" => Self::AudioVolumeUp,
                "AudioVolumeMute" => Self::AudioVolumeMute,
                "MicrophoneToggle" => Self::MicrophoneToggle,
                "MicrophoneVolumeDown" => Self::MicrophoneVolumeDown,
                "MicrophoneVolumeUp" => Self::MicrophoneVolumeUp,
                "MicrophoneVolumeMute" => Self::MicrophoneVolumeMute,
                "SpeechCorrectionList" => Self::SpeechCorrectionList,
                "SpeechInputToggle" => Self::SpeechInputToggle,
                "LaunchApplication1" => Self::LaunchApplication1,
                "LaunchApplication2" => Self::LaunchApplication2,
                "LaunchCalendar" => Self::LaunchCalendar,
                "LaunchContacts" => Self::LaunchContacts,
                "LaunchMail" => Self::LaunchMail,
                "LaunchMediaPlayer" => Self::LaunchMediaPlayer,
                "LaunchMusicPlayer" => Self::LaunchMusicPlayer,
                "LaunchPhone" => Self::LaunchPhone,
                "LaunchScreenSaver" => Self::LaunchScreenSaver,
                "LaunchSpreadsheet" => Self::LaunchSpreadsheet,
                "LaunchWebBrowser" => Self::LaunchWebBrowser,
                "LaunchWebCam" => Self::LaunchWebCam,
                "LaunchWordProcessor" => Self::LaunchWordProcessor,
                "BrowserBack" => Self::BrowserBack,
                "BrowserFavorites" => Self::BrowserFavorites,
                "BrowserForward" => Self::BrowserForward,
                "BrowserHome" => Self::BrowserHome,
                "BrowserRefresh" => Self::BrowserRefresh,
                "BrowserSearch" => Self::BrowserSearch,
                "BrowserStop" => Self::BrowserStop,
                "AppSwitch" => Self::AppSwitch,
                "Call" => Self::Call,
                "Camera" => Self::Camera,
                "CameraFocus" => Self::CameraFocus,
                "EndCall" => Self::EndCall,
                "GoBack" => Self::GoBack,
                "GoHome" => Self::GoHome,
                "HeadsetHook" => Self::HeadsetHook,
                "LastNumberRedial" => Self::LastNumberRedial,
                "Notification" => Self::Notification,
                "MannerMode" => Self::MannerMode,
                "VoiceDial" => Self::VoiceDial,
                "ColorF0Red" => Self::ColorF0Red,
                "ColorF1Green" => Self::ColorF1Green,
                "ColorF2Yellow" => Self::ColorF2Yellow,
                "ColorF3Blue" => Self::ColorF3Blue,
                "ColorF4Grey" => Self::ColorF4Grey,
                "ColorF5Brown" => Self::ColorF5Brown,
                "ClosedCaptionToggle" => Self::ClosedCaptionToggle,
                "Dimmer" => Self::Dimmer,
                "DisplaySwap" => Self::DisplaySwap,
                "Exit" => Self::Exit,
                "FavoriteClear0" => Self::FavoriteClear0,
                "FavoriteClear1" => Self::FavoriteClear1,
                "FavoriteClear2" => Self::FavoriteClear2,
                "FavoriteClear3" => Self::FavoriteClear3,
                "FavoriteRecall0" => Self::FavoriteRecall0,
                "FavoriteRecall1" => Self::FavoriteRecall1,
                "FavoriteRecall2" => Self::FavoriteRecall2,
                "FavoriteRecall3" => Self::FavoriteRecall3,
                "FavoriteStore0" => Self::FavoriteStore0,
                "FavoriteStore1" => Self::FavoriteStore1,
                "FavoriteStore2" => Self::FavoriteStore2,
                "FavoriteStore3" => Self::FavoriteStore3,
                "Guide" => Self::Guide,
                "GuideNextDay" => Self::GuideNextDay,
                "GuidePreviousDay" => Self::GuidePreviousDay,
                "Info" => Self::Info,
                "InstantReplay" => Self::InstantReplay,
                "Link" => Self::Link,
                "ListProgram" => Self::ListProgram,
                "LiveContent" => Self::LiveContent,
                "Lock" => Self::Lock,
                "MediaApps" => Self::MediaApps,
                "MediaAudioTrack" => Self::MediaAudioTrack,
                "MediaLast" => Self::MediaLast,
                "MediaSkipBackward" => Self::MediaSkipBackward,
                "MediaSkipForward" => Self::MediaSkipForward,
                "MediaStepBackward" => Self::MediaStepBackward,
                "MediaStepForward" => Self::MediaStepForward,
                "MediaTopMenu" => Self::MediaTopMenu,
                "NavigateIn" => Self::NavigateIn,
                "NavigateNext" => Self::NavigateNext,
                "NavigateOut" => Self::NavigateOut,
                "NavigatePrevious" => Self::NavigatePrevious,
                "NextFavoriteChannel" => Self::NextFavoriteChannel,
                "NextUserProfile" => Self::NextUserProfile,
                "OnDemand" => Self::OnDemand,
                "Pairing" => Self::Pairing,
                "PinPDown" => Self::PinPDown,
                "PinPMove" => Self::PinPMove,
                "PinPToggle" => Self::PinPToggle,
                "PinPUp" => Self::PinPUp,
                "PlaySpeedDown" => Self::PlaySpeedDown,
                "PlaySpeedReset" => Self::PlaySpeedReset,
                "PlaySpeedUp" => Self::PlaySpeedUp,
                "RandomToggle" => Self::RandomToggle,
                "RcLowBattery" => Self::RcLowBattery,
                "RecordSpeedNext" => Self::RecordSpeedNext,
                "RfBypass" => Self::RfBypass,
                "ScanChannelsToggle" => Self::ScanChannelsToggle,
                "ScreenModeNext" => Self::ScreenModeNext,
                "Settings" => Self::Settings,
                "SplitScreenToggle" => Self::SplitScreenToggle,
                "Subtitle" => Self::Subtitle,
                "Teletext" => Self::Teletext,
                "VideoModeNext" => Self::VideoModeNext,
                "Wink" => Self::Wink,
                "ZoomToggle" => Self::ZoomToggle,
                _ => Self::Unidentified,
            },
        }
    }

    /// Returns the canonical W3C key string for this named key.
    #[must_use]
    pub const fn as_w3c_str(self) -> &'static str {
        match self {
            Self::Unidentified => "Unidentified",
            Self::Alt => "Alt",
            Self::AltGraph => "AltGraph",
            Self::CapsLock => "CapsLock",
            Self::Control => "Control",
            Self::Fn => "Fn",
            Self::FnLock => "FnLock",
            Self::Meta => "Meta",
            Self::NumLock => "NumLock",
            Self::ScrollLock => "ScrollLock",
            Self::Shift => "Shift",
            Self::Symbol => "Symbol",
            Self::SymbolLock => "SymbolLock",
            Self::Hyper => "Hyper",
            Self::Super => "Super",
            Self::Enter => "Enter",
            Self::Tab => "Tab",
            Self::Space => " ",
            Self::ArrowDown => "ArrowDown",
            Self::ArrowLeft => "ArrowLeft",
            Self::ArrowRight => "ArrowRight",
            Self::ArrowUp => "ArrowUp",
            Self::End => "End",
            Self::Home => "Home",
            Self::PageDown => "PageDown",
            Self::PageUp => "PageUp",
            Self::Backspace => "Backspace",
            Self::Clear => "Clear",
            Self::Copy => "Copy",
            Self::CrSel => "CrSel",
            Self::Cut => "Cut",
            Self::Delete => "Delete",
            Self::EraseEof => "EraseEof",
            Self::ExSel => "ExSel",
            Self::Insert => "Insert",
            Self::Paste => "Paste",
            Self::Redo => "Redo",
            Self::Undo => "Undo",
            Self::Accept => "Accept",
            Self::Again => "Again",
            Self::Attn => "Attn",
            Self::Cancel => "Cancel",
            Self::ContextMenu => "ContextMenu",
            Self::Escape => "Escape",
            Self::Execute => "Execute",
            Self::Find => "Find",
            Self::Help => "Help",
            Self::Pause => "Pause",
            Self::Play => "Play",
            Self::Props => "Props",
            Self::Select => "Select",
            Self::ZoomIn => "ZoomIn",
            Self::ZoomOut => "ZoomOut",
            Self::BrightnessDown => "BrightnessDown",
            Self::BrightnessUp => "BrightnessUp",
            Self::Eject => "Eject",
            Self::LogOff => "LogOff",
            Self::Power => "Power",
            Self::PowerOff => "PowerOff",
            Self::PrintScreen => "PrintScreen",
            Self::Hibernate => "Hibernate",
            Self::Standby => "Standby",
            Self::WakeUp => "WakeUp",
            Self::AllCandidates => "AllCandidates",
            Self::Alphanumeric => "Alphanumeric",
            Self::CodeInput => "CodeInput",
            Self::Compose => "Compose",
            Self::Convert => "Convert",
            Self::Dead => "Dead",
            Self::FinalMode => "FinalMode",
            Self::GroupFirst => "GroupFirst",
            Self::GroupLast => "GroupLast",
            Self::GroupNext => "GroupNext",
            Self::GroupPrevious => "GroupPrevious",
            Self::ModeChange => "ModeChange",
            Self::NextCandidate => "NextCandidate",
            Self::NonConvert => "NonConvert",
            Self::PreviousCandidate => "PreviousCandidate",
            Self::Process => "Process",
            Self::SingleCandidate => "SingleCandidate",
            Self::HangulMode => "HangulMode",
            Self::HanjaMode => "HanjaMode",
            Self::JunjaMode => "JunjaMode",
            Self::Eisu => "Eisu",
            Self::Hankaku => "Hankaku",
            Self::Hiragana => "Hiragana",
            Self::HiraganaKatakana => "HiraganaKatakana",
            Self::KanaMode => "KanaMode",
            Self::KanjiMode => "KanjiMode",
            Self::Katakana => "Katakana",
            Self::Romaji => "Romaji",
            Self::Zenkaku => "Zenkaku",
            Self::ZenkakuHankaku => "ZenkakuHankaku",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F11 => "F11",
            Self::F12 => "F12",
            Self::Soft1 => "Soft1",
            Self::Soft2 => "Soft2",
            Self::Soft3 => "Soft3",
            Self::Soft4 => "Soft4",
            Self::ChannelDown => "ChannelDown",
            Self::ChannelUp => "ChannelUp",
            Self::Close => "Close",
            Self::MailForward => "MailForward",
            Self::MailReply => "MailReply",
            Self::MailSend => "MailSend",
            Self::MediaClose => "MediaClose",
            Self::MediaFastForward => "MediaFastForward",
            Self::MediaPause => "MediaPause",
            Self::MediaPlay => "MediaPlay",
            Self::MediaPlayPause => "MediaPlayPause",
            Self::MediaRecord => "MediaRecord",
            Self::MediaRewind => "MediaRewind",
            Self::MediaStop => "MediaStop",
            Self::MediaTrackNext => "MediaTrackNext",
            Self::MediaTrackPrevious => "MediaTrackPrevious",
            Self::New => "New",
            Self::Open => "Open",
            Self::Print => "Print",
            Self::Save => "Save",
            Self::SpellCheck => "SpellCheck",
            Self::Key11 => "11",
            Self::Key12 => "12",
            Self::AudioBalanceLeft => "AudioBalanceLeft",
            Self::AudioBalanceRight => "AudioBalanceRight",
            Self::AudioBassBoostDown => "AudioBassBoostDown",
            Self::AudioBassBoostToggle => "AudioBassBoostToggle",
            Self::AudioBassBoostUp => "AudioBassBoostUp",
            Self::AudioFaderFront => "AudioFaderFront",
            Self::AudioFaderRear => "AudioFaderRear",
            Self::AudioSurroundModeNext => "AudioSurroundModeNext",
            Self::AudioTrebleDown => "AudioTrebleDown",
            Self::AudioTrebleUp => "AudioTrebleUp",
            Self::AudioVolumeDown => "AudioVolumeDown",
            Self::AudioVolumeUp => "AudioVolumeUp",
            Self::AudioVolumeMute => "AudioVolumeMute",
            Self::MicrophoneToggle => "MicrophoneToggle",
            Self::MicrophoneVolumeDown => "MicrophoneVolumeDown",
            Self::MicrophoneVolumeUp => "MicrophoneVolumeUp",
            Self::MicrophoneVolumeMute => "MicrophoneVolumeMute",
            Self::SpeechCorrectionList => "SpeechCorrectionList",
            Self::SpeechInputToggle => "SpeechInputToggle",
            Self::LaunchApplication1 => "LaunchApplication1",
            Self::LaunchApplication2 => "LaunchApplication2",
            Self::LaunchCalendar => "LaunchCalendar",
            Self::LaunchContacts => "LaunchContacts",
            Self::LaunchMail => "LaunchMail",
            Self::LaunchMediaPlayer => "LaunchMediaPlayer",
            Self::LaunchMusicPlayer => "LaunchMusicPlayer",
            Self::LaunchPhone => "LaunchPhone",
            Self::LaunchScreenSaver => "LaunchScreenSaver",
            Self::LaunchSpreadsheet => "LaunchSpreadsheet",
            Self::LaunchWebBrowser => "LaunchWebBrowser",
            Self::LaunchWebCam => "LaunchWebCam",
            Self::LaunchWordProcessor => "LaunchWordProcessor",
            Self::BrowserBack => "BrowserBack",
            Self::BrowserFavorites => "BrowserFavorites",
            Self::BrowserForward => "BrowserForward",
            Self::BrowserHome => "BrowserHome",
            Self::BrowserRefresh => "BrowserRefresh",
            Self::BrowserSearch => "BrowserSearch",
            Self::BrowserStop => "BrowserStop",
            Self::AppSwitch => "AppSwitch",
            Self::Call => "Call",
            Self::Camera => "Camera",
            Self::CameraFocus => "CameraFocus",
            Self::EndCall => "EndCall",
            Self::GoBack => "GoBack",
            Self::GoHome => "GoHome",
            Self::HeadsetHook => "HeadsetHook",
            Self::LastNumberRedial => "LastNumberRedial",
            Self::Notification => "Notification",
            Self::MannerMode => "MannerMode",
            Self::VoiceDial => "VoiceDial",
            Self::Tv => "TV",
            Self::Tv3DMode => "TV3DMode",
            Self::TvAntennaCable => "TVAntennaCable",
            Self::TvAudioDescription => "TVAudioDescription",
            Self::TvAudioDescriptionMixDown => "TVAudioDescriptionMixDown",
            Self::TvAudioDescriptionMixUp => "TVAudioDescriptionMixUp",
            Self::TvContentsMenu => "TVContentsMenu",
            Self::TvDataService => "TVDataService",
            Self::TvInput => "TVInput",
            Self::TvInputComponent1 => "TVInputComponent1",
            Self::TvInputComponent2 => "TVInputComponent2",
            Self::TvInputComposite1 => "TVInputComposite1",
            Self::TvInputComposite2 => "TVInputComposite2",
            Self::TvInputHdmi1 => "TVInputHDMI1",
            Self::TvInputHdmi2 => "TVInputHDMI2",
            Self::TvInputHdmi3 => "TVInputHDMI3",
            Self::TvInputHdmi4 => "TVInputHDMI4",
            Self::TvInputVga1 => "TVInputVGA1",
            Self::TvMediaContext => "TVMediaContext",
            Self::TvNetwork => "TVNetwork",
            Self::TvNumberEntry => "TVNumberEntry",
            Self::TvPower => "TVPower",
            Self::TvRadioService => "TVRadioService",
            Self::TvSatellite => "TVSatellite",
            Self::TvSatelliteBS => "TVSatelliteBS",
            Self::TvSatelliteCS => "TVSatelliteCS",
            Self::TvSatelliteToggle => "TVSatelliteToggle",
            Self::TvTerrestrialAnalog => "TVTerrestrialAnalog",
            Self::TvTerrestrialDigital => "TVTerrestrialDigital",
            Self::TvTimer => "TVTimer",
            Self::AvrInput => "AVRInput",
            Self::AvrPower => "AVRPower",
            Self::ColorF0Red => "ColorF0Red",
            Self::ColorF1Green => "ColorF1Green",
            Self::ColorF2Yellow => "ColorF2Yellow",
            Self::ColorF3Blue => "ColorF3Blue",
            Self::ColorF4Grey => "ColorF4Grey",
            Self::ColorF5Brown => "ColorF5Brown",
            Self::ClosedCaptionToggle => "ClosedCaptionToggle",
            Self::Dimmer => "Dimmer",
            Self::DisplaySwap => "DisplaySwap",
            Self::Dvr => "DVR",
            Self::Exit => "Exit",
            Self::FavoriteClear0 => "FavoriteClear0",
            Self::FavoriteClear1 => "FavoriteClear1",
            Self::FavoriteClear2 => "FavoriteClear2",
            Self::FavoriteClear3 => "FavoriteClear3",
            Self::FavoriteRecall0 => "FavoriteRecall0",
            Self::FavoriteRecall1 => "FavoriteRecall1",
            Self::FavoriteRecall2 => "FavoriteRecall2",
            Self::FavoriteRecall3 => "FavoriteRecall3",
            Self::FavoriteStore0 => "FavoriteStore0",
            Self::FavoriteStore1 => "FavoriteStore1",
            Self::FavoriteStore2 => "FavoriteStore2",
            Self::FavoriteStore3 => "FavoriteStore3",
            Self::Guide => "Guide",
            Self::GuideNextDay => "GuideNextDay",
            Self::GuidePreviousDay => "GuidePreviousDay",
            Self::Info => "Info",
            Self::InstantReplay => "InstantReplay",
            Self::Link => "Link",
            Self::ListProgram => "ListProgram",
            Self::LiveContent => "LiveContent",
            Self::Lock => "Lock",
            Self::MediaApps => "MediaApps",
            Self::MediaAudioTrack => "MediaAudioTrack",
            Self::MediaLast => "MediaLast",
            Self::MediaSkipBackward => "MediaSkipBackward",
            Self::MediaSkipForward => "MediaSkipForward",
            Self::MediaStepBackward => "MediaStepBackward",
            Self::MediaStepForward => "MediaStepForward",
            Self::MediaTopMenu => "MediaTopMenu",
            Self::NavigateIn => "NavigateIn",
            Self::NavigateNext => "NavigateNext",
            Self::NavigateOut => "NavigateOut",
            Self::NavigatePrevious => "NavigatePrevious",
            Self::NextFavoriteChannel => "NextFavoriteChannel",
            Self::NextUserProfile => "NextUserProfile",
            Self::OnDemand => "OnDemand",
            Self::Pairing => "Pairing",
            Self::PinPDown => "PinPDown",
            Self::PinPMove => "PinPMove",
            Self::PinPToggle => "PinPToggle",
            Self::PinPUp => "PinPUp",
            Self::PlaySpeedDown => "PlaySpeedDown",
            Self::PlaySpeedReset => "PlaySpeedReset",
            Self::PlaySpeedUp => "PlaySpeedUp",
            Self::RandomToggle => "RandomToggle",
            Self::RcLowBattery => "RcLowBattery",
            Self::RecordSpeedNext => "RecordSpeedNext",
            Self::RfBypass => "RfBypass",
            Self::ScanChannelsToggle => "ScanChannelsToggle",
            Self::ScreenModeNext => "ScreenModeNext",
            Self::Settings => "Settings",
            Self::SplitScreenToggle => "SplitScreenToggle",
            Self::StbInput => "STBInput",
            Self::StbPower => "STBPower",
            Self::Subtitle => "Subtitle",
            Self::Teletext => "Teletext",
            Self::VideoModeNext => "VideoModeNext",
            Self::Wink => "Wink",
            Self::ZoomToggle => "ZoomToggle",
        }
    }
}

/// Snapshot of the current modality state for a single provider root.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModalitySnapshot {
    /// The most recent input modality observed for this context.
    pub last_pointer_type: Option<PointerType>,
    /// Whether any press interaction is currently active for this context.
    pub global_press_active: bool,
}

impl ModalitySnapshot {
    /// Returns whether the snapshot represents a physical pointer interaction.
    #[must_use]
    pub const fn had_pointer_interaction(self) -> bool {
        matches!(
            self.last_pointer_type,
            Some(PointerType::Mouse | PointerType::Touch | PointerType::Pen)
        )
    }
}

/// Shared, instance-scoped modality contract.
///
/// Requires `Send + Sync` so implementations can be wrapped in
/// [`ArsRc`](crate::ArsRc) and safely shared across threads on native targets.
/// On wasm (single-threaded), `Send + Sync` is trivially satisfied.
pub trait ModalityContext: Send + Sync {
    /// Returns a copy of the current modality snapshot.
    fn snapshot(&self) -> ModalitySnapshot;

    /// Returns the most recent input modality observed by this context.
    fn last_pointer_type(&self) -> Option<PointerType> {
        self.snapshot().last_pointer_type
    }

    /// Returns whether the context currently reflects a pointer interaction.
    fn had_pointer_interaction(&self) -> bool {
        self.snapshot().had_pointer_interaction()
    }

    /// Returns whether any press interaction is currently active.
    fn is_global_press_active(&self) -> bool {
        self.snapshot().global_press_active
    }

    /// Records a keyboard-driven interaction.
    fn on_key_down(&self, key: KeyboardKey, modifiers: KeyModifiers);

    /// Records a pointer-driven interaction.
    fn on_pointer_down(&self, pointer_type: PointerType);

    /// Records a virtual or programmatic interaction source.
    fn on_virtual_input(&self);

    /// Sets the global press-active flag.
    fn set_global_press_active(&self, active: bool);

    /// Clears all modality state for deterministic tests and provider resets.
    fn clear(&self);
}

/// Default interior-mutable modality context implementation.
///
/// Uses [`AtomicU8`] and [`AtomicBool`] for lock-free interior mutability.
/// On wasm (single-threaded), atomic operations compile to plain memory
/// reads/writes with zero overhead.
pub struct DefaultModalityContext {
    /// Encoded `Option<PointerType>`: 0=None, 1=Mouse, 2=Touch, 3=Pen,
    /// 4=Keyboard, 5=Virtual.
    last_pointer_type: AtomicU8,
    global_press_active: AtomicBool,
}

impl DefaultModalityContext {
    /// Creates a new context with no prior modality and no active press state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_pointer_type: AtomicU8::new(0),
            global_press_active: AtomicBool::new(false),
        }
    }
}

impl Default for DefaultModalityContext {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for DefaultModalityContext {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DefaultModalityContext")
            .field("snapshot", &self.snapshot())
            .finish()
    }
}

/// Encode `Option<PointerType>` as a `u8` for atomic storage.
const fn pointer_type_to_u8(pt: Option<PointerType>) -> u8 {
    match pt {
        None => 0,
        Some(PointerType::Mouse) => 1,
        Some(PointerType::Touch) => 2,
        Some(PointerType::Pen) => 3,
        Some(PointerType::Keyboard) => 4,
        Some(PointerType::Virtual) => 5,
    }
}

/// Decode a `u8` back to `Option<PointerType>`.
const fn u8_to_pointer_type(v: u8) -> Option<PointerType> {
    match v {
        1 => Some(PointerType::Mouse),
        2 => Some(PointerType::Touch),
        3 => Some(PointerType::Pen),
        4 => Some(PointerType::Keyboard),
        5 => Some(PointerType::Virtual),
        _ => None,
    }
}

impl ModalityContext for DefaultModalityContext {
    fn snapshot(&self) -> ModalitySnapshot {
        ModalitySnapshot {
            last_pointer_type: u8_to_pointer_type(self.last_pointer_type.load(Ordering::Relaxed)),
            global_press_active: self.global_press_active.load(Ordering::Relaxed),
        }
    }

    fn on_key_down(&self, _key: KeyboardKey, _modifiers: KeyModifiers) {
        self.last_pointer_type.store(
            pointer_type_to_u8(Some(PointerType::Keyboard)),
            Ordering::Relaxed,
        );
    }

    fn on_pointer_down(&self, pointer_type: PointerType) {
        self.last_pointer_type
            .store(pointer_type_to_u8(Some(pointer_type)), Ordering::Relaxed);
    }

    fn on_virtual_input(&self) {
        self.last_pointer_type.store(
            pointer_type_to_u8(Some(PointerType::Virtual)),
            Ordering::Relaxed,
        );
    }

    fn set_global_press_active(&self, active: bool) {
        self.global_press_active.store(active, Ordering::Relaxed);
    }

    fn clear(&self) {
        self.last_pointer_type.store(0, Ordering::Relaxed);
        self.global_press_active.store(false, Ordering::Relaxed);
    }
}

/// No-op modality context for SSR and tests that intentionally disable tracking.
#[derive(Clone, Copy, Debug, Default)]
pub struct NullModalityContext;

impl ModalityContext for NullModalityContext {
    fn snapshot(&self) -> ModalitySnapshot {
        ModalitySnapshot::default()
    }

    fn on_key_down(&self, _key: KeyboardKey, _modifiers: KeyModifiers) {}

    fn on_pointer_down(&self, _pointer_type: PointerType) {}

    fn on_virtual_input(&self) {}

    fn set_global_press_active(&self, _active: bool) {}

    fn clear(&self) {}
}

// ── ArsRc<dyn ModalityContext> constructor ──────────────────────────

impl crate::ArsRc<dyn ModalityContext> {
    /// Creates a trait-object `ArsRc` from any [`ModalityContext`] implementation.
    ///
    /// This enables erased construction without requiring nightly `CoerceUnsized`:
    /// ```ignore
    /// let ctx: ArsRc<dyn ModalityContext> = ArsRc::from_modality(DefaultModalityContext::new());
    /// ```
    pub fn from_modality(value: impl ModalityContext + 'static) -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Self(alloc::rc::Rc::new(value))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self(alloc::sync::Arc::new(value))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyboard_key_round_trips_across_w3c_registry() {
        let cases = [
            (KeyboardKey::Unidentified, "Unidentified"),
            (KeyboardKey::Alt, "Alt"),
            (KeyboardKey::AltGraph, "AltGraph"),
            (KeyboardKey::CapsLock, "CapsLock"),
            (KeyboardKey::Control, "Control"),
            (KeyboardKey::Fn, "Fn"),
            (KeyboardKey::FnLock, "FnLock"),
            (KeyboardKey::Meta, "Meta"),
            (KeyboardKey::NumLock, "NumLock"),
            (KeyboardKey::ScrollLock, "ScrollLock"),
            (KeyboardKey::Shift, "Shift"),
            (KeyboardKey::Symbol, "Symbol"),
            (KeyboardKey::SymbolLock, "SymbolLock"),
            (KeyboardKey::Hyper, "Hyper"),
            (KeyboardKey::Super, "Super"),
            (KeyboardKey::Enter, "Enter"),
            (KeyboardKey::Tab, "Tab"),
            (KeyboardKey::Space, " "),
            (KeyboardKey::ArrowDown, "ArrowDown"),
            (KeyboardKey::ArrowLeft, "ArrowLeft"),
            (KeyboardKey::ArrowRight, "ArrowRight"),
            (KeyboardKey::ArrowUp, "ArrowUp"),
            (KeyboardKey::End, "End"),
            (KeyboardKey::Home, "Home"),
            (KeyboardKey::PageDown, "PageDown"),
            (KeyboardKey::PageUp, "PageUp"),
            (KeyboardKey::Backspace, "Backspace"),
            (KeyboardKey::Clear, "Clear"),
            (KeyboardKey::Copy, "Copy"),
            (KeyboardKey::CrSel, "CrSel"),
            (KeyboardKey::Cut, "Cut"),
            (KeyboardKey::Delete, "Delete"),
            (KeyboardKey::EraseEof, "EraseEof"),
            (KeyboardKey::ExSel, "ExSel"),
            (KeyboardKey::Insert, "Insert"),
            (KeyboardKey::Paste, "Paste"),
            (KeyboardKey::Redo, "Redo"),
            (KeyboardKey::Undo, "Undo"),
            (KeyboardKey::Accept, "Accept"),
            (KeyboardKey::Again, "Again"),
            (KeyboardKey::Attn, "Attn"),
            (KeyboardKey::Cancel, "Cancel"),
            (KeyboardKey::ContextMenu, "ContextMenu"),
            (KeyboardKey::Escape, "Escape"),
            (KeyboardKey::Execute, "Execute"),
            (KeyboardKey::Find, "Find"),
            (KeyboardKey::Help, "Help"),
            (KeyboardKey::Pause, "Pause"),
            (KeyboardKey::Play, "Play"),
            (KeyboardKey::Props, "Props"),
            (KeyboardKey::Select, "Select"),
            (KeyboardKey::ZoomIn, "ZoomIn"),
            (KeyboardKey::ZoomOut, "ZoomOut"),
            (KeyboardKey::BrightnessDown, "BrightnessDown"),
            (KeyboardKey::BrightnessUp, "BrightnessUp"),
            (KeyboardKey::Eject, "Eject"),
            (KeyboardKey::LogOff, "LogOff"),
            (KeyboardKey::Power, "Power"),
            (KeyboardKey::PowerOff, "PowerOff"),
            (KeyboardKey::PrintScreen, "PrintScreen"),
            (KeyboardKey::Hibernate, "Hibernate"),
            (KeyboardKey::Standby, "Standby"),
            (KeyboardKey::WakeUp, "WakeUp"),
            (KeyboardKey::AllCandidates, "AllCandidates"),
            (KeyboardKey::Alphanumeric, "Alphanumeric"),
            (KeyboardKey::CodeInput, "CodeInput"),
            (KeyboardKey::Compose, "Compose"),
            (KeyboardKey::Convert, "Convert"),
            (KeyboardKey::Dead, "Dead"),
            (KeyboardKey::FinalMode, "FinalMode"),
            (KeyboardKey::GroupFirst, "GroupFirst"),
            (KeyboardKey::GroupLast, "GroupLast"),
            (KeyboardKey::GroupNext, "GroupNext"),
            (KeyboardKey::GroupPrevious, "GroupPrevious"),
            (KeyboardKey::ModeChange, "ModeChange"),
            (KeyboardKey::NextCandidate, "NextCandidate"),
            (KeyboardKey::NonConvert, "NonConvert"),
            (KeyboardKey::PreviousCandidate, "PreviousCandidate"),
            (KeyboardKey::Process, "Process"),
            (KeyboardKey::SingleCandidate, "SingleCandidate"),
            (KeyboardKey::HangulMode, "HangulMode"),
            (KeyboardKey::HanjaMode, "HanjaMode"),
            (KeyboardKey::JunjaMode, "JunjaMode"),
            (KeyboardKey::Eisu, "Eisu"),
            (KeyboardKey::Hankaku, "Hankaku"),
            (KeyboardKey::Hiragana, "Hiragana"),
            (KeyboardKey::HiraganaKatakana, "HiraganaKatakana"),
            (KeyboardKey::KanaMode, "KanaMode"),
            (KeyboardKey::KanjiMode, "KanjiMode"),
            (KeyboardKey::Katakana, "Katakana"),
            (KeyboardKey::Romaji, "Romaji"),
            (KeyboardKey::Zenkaku, "Zenkaku"),
            (KeyboardKey::ZenkakuHankaku, "ZenkakuHankaku"),
            (KeyboardKey::F1, "F1"),
            (KeyboardKey::F2, "F2"),
            (KeyboardKey::F3, "F3"),
            (KeyboardKey::F4, "F4"),
            (KeyboardKey::F5, "F5"),
            (KeyboardKey::F6, "F6"),
            (KeyboardKey::F7, "F7"),
            (KeyboardKey::F8, "F8"),
            (KeyboardKey::F9, "F9"),
            (KeyboardKey::F10, "F10"),
            (KeyboardKey::F11, "F11"),
            (KeyboardKey::F12, "F12"),
            (KeyboardKey::Soft1, "Soft1"),
            (KeyboardKey::Soft2, "Soft2"),
            (KeyboardKey::Soft3, "Soft3"),
            (KeyboardKey::Soft4, "Soft4"),
            (KeyboardKey::ChannelDown, "ChannelDown"),
            (KeyboardKey::ChannelUp, "ChannelUp"),
            (KeyboardKey::Close, "Close"),
            (KeyboardKey::MailForward, "MailForward"),
            (KeyboardKey::MailReply, "MailReply"),
            (KeyboardKey::MailSend, "MailSend"),
            (KeyboardKey::MediaClose, "MediaClose"),
            (KeyboardKey::MediaFastForward, "MediaFastForward"),
            (KeyboardKey::MediaPause, "MediaPause"),
            (KeyboardKey::MediaPlay, "MediaPlay"),
            (KeyboardKey::MediaPlayPause, "MediaPlayPause"),
            (KeyboardKey::MediaRecord, "MediaRecord"),
            (KeyboardKey::MediaRewind, "MediaRewind"),
            (KeyboardKey::MediaStop, "MediaStop"),
            (KeyboardKey::MediaTrackNext, "MediaTrackNext"),
            (KeyboardKey::MediaTrackPrevious, "MediaTrackPrevious"),
            (KeyboardKey::New, "New"),
            (KeyboardKey::Open, "Open"),
            (KeyboardKey::Print, "Print"),
            (KeyboardKey::Save, "Save"),
            (KeyboardKey::SpellCheck, "SpellCheck"),
            (KeyboardKey::Key11, "11"),
            (KeyboardKey::Key12, "12"),
            (KeyboardKey::AudioBalanceLeft, "AudioBalanceLeft"),
            (KeyboardKey::AudioBalanceRight, "AudioBalanceRight"),
            (KeyboardKey::AudioBassBoostDown, "AudioBassBoostDown"),
            (KeyboardKey::AudioBassBoostToggle, "AudioBassBoostToggle"),
            (KeyboardKey::AudioBassBoostUp, "AudioBassBoostUp"),
            (KeyboardKey::AudioFaderFront, "AudioFaderFront"),
            (KeyboardKey::AudioFaderRear, "AudioFaderRear"),
            (KeyboardKey::AudioSurroundModeNext, "AudioSurroundModeNext"),
            (KeyboardKey::AudioTrebleDown, "AudioTrebleDown"),
            (KeyboardKey::AudioTrebleUp, "AudioTrebleUp"),
            (KeyboardKey::AudioVolumeDown, "AudioVolumeDown"),
            (KeyboardKey::AudioVolumeUp, "AudioVolumeUp"),
            (KeyboardKey::AudioVolumeMute, "AudioVolumeMute"),
            (KeyboardKey::MicrophoneToggle, "MicrophoneToggle"),
            (KeyboardKey::MicrophoneVolumeDown, "MicrophoneVolumeDown"),
            (KeyboardKey::MicrophoneVolumeUp, "MicrophoneVolumeUp"),
            (KeyboardKey::MicrophoneVolumeMute, "MicrophoneVolumeMute"),
            (KeyboardKey::SpeechCorrectionList, "SpeechCorrectionList"),
            (KeyboardKey::SpeechInputToggle, "SpeechInputToggle"),
            (KeyboardKey::LaunchApplication1, "LaunchApplication1"),
            (KeyboardKey::LaunchApplication2, "LaunchApplication2"),
            (KeyboardKey::LaunchCalendar, "LaunchCalendar"),
            (KeyboardKey::LaunchContacts, "LaunchContacts"),
            (KeyboardKey::LaunchMail, "LaunchMail"),
            (KeyboardKey::LaunchMediaPlayer, "LaunchMediaPlayer"),
            (KeyboardKey::LaunchMusicPlayer, "LaunchMusicPlayer"),
            (KeyboardKey::LaunchPhone, "LaunchPhone"),
            (KeyboardKey::LaunchScreenSaver, "LaunchScreenSaver"),
            (KeyboardKey::LaunchSpreadsheet, "LaunchSpreadsheet"),
            (KeyboardKey::LaunchWebBrowser, "LaunchWebBrowser"),
            (KeyboardKey::LaunchWebCam, "LaunchWebCam"),
            (KeyboardKey::LaunchWordProcessor, "LaunchWordProcessor"),
            (KeyboardKey::BrowserBack, "BrowserBack"),
            (KeyboardKey::BrowserFavorites, "BrowserFavorites"),
            (KeyboardKey::BrowserForward, "BrowserForward"),
            (KeyboardKey::BrowserHome, "BrowserHome"),
            (KeyboardKey::BrowserRefresh, "BrowserRefresh"),
            (KeyboardKey::BrowserSearch, "BrowserSearch"),
            (KeyboardKey::BrowserStop, "BrowserStop"),
            (KeyboardKey::AppSwitch, "AppSwitch"),
            (KeyboardKey::Call, "Call"),
            (KeyboardKey::Camera, "Camera"),
            (KeyboardKey::CameraFocus, "CameraFocus"),
            (KeyboardKey::EndCall, "EndCall"),
            (KeyboardKey::GoBack, "GoBack"),
            (KeyboardKey::GoHome, "GoHome"),
            (KeyboardKey::HeadsetHook, "HeadsetHook"),
            (KeyboardKey::LastNumberRedial, "LastNumberRedial"),
            (KeyboardKey::Notification, "Notification"),
            (KeyboardKey::MannerMode, "MannerMode"),
            (KeyboardKey::VoiceDial, "VoiceDial"),
            (KeyboardKey::Tv, "TV"),
            (KeyboardKey::Tv3DMode, "TV3DMode"),
            (KeyboardKey::TvAntennaCable, "TVAntennaCable"),
            (KeyboardKey::TvAudioDescription, "TVAudioDescription"),
            (
                KeyboardKey::TvAudioDescriptionMixDown,
                "TVAudioDescriptionMixDown",
            ),
            (
                KeyboardKey::TvAudioDescriptionMixUp,
                "TVAudioDescriptionMixUp",
            ),
            (KeyboardKey::TvContentsMenu, "TVContentsMenu"),
            (KeyboardKey::TvDataService, "TVDataService"),
            (KeyboardKey::TvInput, "TVInput"),
            (KeyboardKey::TvInputComponent1, "TVInputComponent1"),
            (KeyboardKey::TvInputComponent2, "TVInputComponent2"),
            (KeyboardKey::TvInputComposite1, "TVInputComposite1"),
            (KeyboardKey::TvInputComposite2, "TVInputComposite2"),
            (KeyboardKey::TvInputHdmi1, "TVInputHDMI1"),
            (KeyboardKey::TvInputHdmi2, "TVInputHDMI2"),
            (KeyboardKey::TvInputHdmi3, "TVInputHDMI3"),
            (KeyboardKey::TvInputHdmi4, "TVInputHDMI4"),
            (KeyboardKey::TvInputVga1, "TVInputVGA1"),
            (KeyboardKey::TvMediaContext, "TVMediaContext"),
            (KeyboardKey::TvNetwork, "TVNetwork"),
            (KeyboardKey::TvNumberEntry, "TVNumberEntry"),
            (KeyboardKey::TvPower, "TVPower"),
            (KeyboardKey::TvRadioService, "TVRadioService"),
            (KeyboardKey::TvSatellite, "TVSatellite"),
            (KeyboardKey::TvSatelliteBS, "TVSatelliteBS"),
            (KeyboardKey::TvSatelliteCS, "TVSatelliteCS"),
            (KeyboardKey::TvSatelliteToggle, "TVSatelliteToggle"),
            (KeyboardKey::TvTerrestrialAnalog, "TVTerrestrialAnalog"),
            (KeyboardKey::TvTerrestrialDigital, "TVTerrestrialDigital"),
            (KeyboardKey::TvTimer, "TVTimer"),
            (KeyboardKey::AvrInput, "AVRInput"),
            (KeyboardKey::AvrPower, "AVRPower"),
            (KeyboardKey::ColorF0Red, "ColorF0Red"),
            (KeyboardKey::ColorF1Green, "ColorF1Green"),
            (KeyboardKey::ColorF2Yellow, "ColorF2Yellow"),
            (KeyboardKey::ColorF3Blue, "ColorF3Blue"),
            (KeyboardKey::ColorF4Grey, "ColorF4Grey"),
            (KeyboardKey::ColorF5Brown, "ColorF5Brown"),
            (KeyboardKey::ClosedCaptionToggle, "ClosedCaptionToggle"),
            (KeyboardKey::Dimmer, "Dimmer"),
            (KeyboardKey::DisplaySwap, "DisplaySwap"),
            (KeyboardKey::Dvr, "DVR"),
            (KeyboardKey::Exit, "Exit"),
            (KeyboardKey::FavoriteClear0, "FavoriteClear0"),
            (KeyboardKey::FavoriteClear1, "FavoriteClear1"),
            (KeyboardKey::FavoriteClear2, "FavoriteClear2"),
            (KeyboardKey::FavoriteClear3, "FavoriteClear3"),
            (KeyboardKey::FavoriteRecall0, "FavoriteRecall0"),
            (KeyboardKey::FavoriteRecall1, "FavoriteRecall1"),
            (KeyboardKey::FavoriteRecall2, "FavoriteRecall2"),
            (KeyboardKey::FavoriteRecall3, "FavoriteRecall3"),
            (KeyboardKey::FavoriteStore0, "FavoriteStore0"),
            (KeyboardKey::FavoriteStore1, "FavoriteStore1"),
            (KeyboardKey::FavoriteStore2, "FavoriteStore2"),
            (KeyboardKey::FavoriteStore3, "FavoriteStore3"),
            (KeyboardKey::Guide, "Guide"),
            (KeyboardKey::GuideNextDay, "GuideNextDay"),
            (KeyboardKey::GuidePreviousDay, "GuidePreviousDay"),
            (KeyboardKey::Info, "Info"),
            (KeyboardKey::InstantReplay, "InstantReplay"),
            (KeyboardKey::Link, "Link"),
            (KeyboardKey::ListProgram, "ListProgram"),
            (KeyboardKey::LiveContent, "LiveContent"),
            (KeyboardKey::Lock, "Lock"),
            (KeyboardKey::MediaApps, "MediaApps"),
            (KeyboardKey::MediaAudioTrack, "MediaAudioTrack"),
            (KeyboardKey::MediaLast, "MediaLast"),
            (KeyboardKey::MediaSkipBackward, "MediaSkipBackward"),
            (KeyboardKey::MediaSkipForward, "MediaSkipForward"),
            (KeyboardKey::MediaStepBackward, "MediaStepBackward"),
            (KeyboardKey::MediaStepForward, "MediaStepForward"),
            (KeyboardKey::MediaTopMenu, "MediaTopMenu"),
            (KeyboardKey::NavigateIn, "NavigateIn"),
            (KeyboardKey::NavigateNext, "NavigateNext"),
            (KeyboardKey::NavigateOut, "NavigateOut"),
            (KeyboardKey::NavigatePrevious, "NavigatePrevious"),
            (KeyboardKey::NextFavoriteChannel, "NextFavoriteChannel"),
            (KeyboardKey::NextUserProfile, "NextUserProfile"),
            (KeyboardKey::OnDemand, "OnDemand"),
            (KeyboardKey::Pairing, "Pairing"),
            (KeyboardKey::PinPDown, "PinPDown"),
            (KeyboardKey::PinPMove, "PinPMove"),
            (KeyboardKey::PinPToggle, "PinPToggle"),
            (KeyboardKey::PinPUp, "PinPUp"),
            (KeyboardKey::PlaySpeedDown, "PlaySpeedDown"),
            (KeyboardKey::PlaySpeedReset, "PlaySpeedReset"),
            (KeyboardKey::PlaySpeedUp, "PlaySpeedUp"),
            (KeyboardKey::RandomToggle, "RandomToggle"),
            (KeyboardKey::RcLowBattery, "RcLowBattery"),
            (KeyboardKey::RecordSpeedNext, "RecordSpeedNext"),
            (KeyboardKey::RfBypass, "RfBypass"),
            (KeyboardKey::ScanChannelsToggle, "ScanChannelsToggle"),
            (KeyboardKey::ScreenModeNext, "ScreenModeNext"),
            (KeyboardKey::Settings, "Settings"),
            (KeyboardKey::SplitScreenToggle, "SplitScreenToggle"),
            (KeyboardKey::StbInput, "STBInput"),
            (KeyboardKey::StbPower, "STBPower"),
            (KeyboardKey::Subtitle, "Subtitle"),
            (KeyboardKey::Teletext, "Teletext"),
            (KeyboardKey::VideoModeNext, "VideoModeNext"),
            (KeyboardKey::Wink, "Wink"),
            (KeyboardKey::ZoomToggle, "ZoomToggle"),
        ];

        for (key, value) in cases {
            assert_eq!(KeyboardKey::from_key_str(value), key);
            assert_eq!(key.as_w3c_str(), value);
        }
    }

    #[test]
    fn keyboard_key_rejects_printable_and_unknown_values() {
        assert_eq!(KeyboardKey::from_key_str("a"), KeyboardKey::Unidentified);
        assert_eq!(
            KeyboardKey::from_key_str("Enter "),
            KeyboardKey::Unidentified
        );
        assert_eq!(
            KeyboardKey::from_key_str("UnknownKey"),
            KeyboardKey::Unidentified
        );
    }

    #[test]
    fn default_modality_context_starts_empty() {
        let context = DefaultModalityContext::new();

        assert_eq!(context.last_pointer_type(), None);
        assert!(!context.had_pointer_interaction());
        assert!(!context.is_global_press_active());
    }

    #[test]
    fn keyboard_input_sets_keyboard_modality() {
        let context = DefaultModalityContext::new();
        context.on_key_down(KeyboardKey::Tab, KeyModifiers::default());

        assert_eq!(context.last_pointer_type(), Some(PointerType::Keyboard));
        assert!(!context.had_pointer_interaction());
    }

    #[test]
    fn pointer_input_tracks_pointer_modalities() {
        let context = DefaultModalityContext::new();

        context.on_pointer_down(PointerType::Mouse);
        assert_eq!(context.last_pointer_type(), Some(PointerType::Mouse));
        assert!(context.had_pointer_interaction());

        context.on_pointer_down(PointerType::Touch);
        assert_eq!(context.last_pointer_type(), Some(PointerType::Touch));
        assert!(context.had_pointer_interaction());

        context.on_pointer_down(PointerType::Pen);
        assert_eq!(context.last_pointer_type(), Some(PointerType::Pen));
        assert!(context.had_pointer_interaction());
    }

    #[test]
    fn virtual_input_is_not_pointer_interaction() {
        let context = DefaultModalityContext::new();
        context.on_virtual_input();

        assert_eq!(context.last_pointer_type(), Some(PointerType::Virtual));
        assert!(!context.had_pointer_interaction());
    }

    #[test]
    fn global_press_active_toggles() {
        let context = DefaultModalityContext::new();

        context.set_global_press_active(true);
        assert!(context.is_global_press_active());

        context.set_global_press_active(false);
        assert!(!context.is_global_press_active());
    }

    #[test]
    fn contexts_do_not_share_state() {
        let left = DefaultModalityContext::new();
        let right = DefaultModalityContext::new();

        left.on_pointer_down(PointerType::Mouse);

        assert_eq!(left.last_pointer_type(), Some(PointerType::Mouse));
        assert_eq!(right.last_pointer_type(), None);
    }

    #[test]
    fn clear_resets_context_state() {
        let context = DefaultModalityContext::new();
        context.on_pointer_down(PointerType::Touch);
        context.set_global_press_active(true);

        context.clear();

        assert_eq!(context.snapshot(), ModalitySnapshot::default());
    }

    #[test]
    fn null_modality_context_is_a_no_op() {
        let context = NullModalityContext;
        context.on_key_down(KeyboardKey::Tab, KeyModifiers::default());
        context.on_pointer_down(PointerType::Mouse);
        context.on_virtual_input();
        context.set_global_press_active(true);
        context.clear();

        assert_eq!(context.snapshot(), ModalitySnapshot::default());
    }

    #[test]
    fn default_modality_context_default_trait() {
        let context = DefaultModalityContext::default();
        assert_eq!(context.snapshot(), ModalitySnapshot::default());
    }

    #[test]
    fn default_modality_context_debug_output() {
        #[cfg(not(feature = "std"))]
        use alloc::format;

        let context = DefaultModalityContext::new();
        let debug = format!("{context:?}");
        assert!(debug.contains("DefaultModalityContext"));
        assert!(debug.contains("snapshot"));
    }

    #[test]
    fn pointer_type_encoding_covers_none() {
        // Exercises the None → 0 path in pointer_type_to_u8 via clear()
        let context = DefaultModalityContext::new();
        context.on_pointer_down(PointerType::Mouse);
        context.clear();
        assert_eq!(context.last_pointer_type(), None);
    }
}
