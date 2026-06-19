pub struct Color {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

impl Color {
    pub fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    pub fn as_wgpu_color(&self) -> wgpu::Color {
        wgpu::Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: self.a,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyCode {
    /// <kbd>`</kbd> on a US keyboard. This is also called a backtick or grave.
    /// This is the <kbd>半角</kbd>/<kbd>全角</kbd>/<kbd>漢字</kbd>
    /// (hankaku/zenkaku/kanji) key on Japanese keyboards
    Backquote,
    /// Used for both the US <kbd>\\</kbd> (on the 101-key layout) and also for the key
    /// located between the <kbd>"</kbd> and <kbd>Enter</kbd> keys on row C of the 102-,
    /// 104- and 106-key layouts.
    /// Labeled <kbd>#</kbd> on a UK (102) keyboard.
    Backslash,
    /// <kbd>[</kbd> on a US keyboard.
    BracketLeft,
    /// <kbd>]</kbd> on a US keyboard.
    BracketRight,
    /// <kbd>,</kbd> on a US keyboard.
    Comma,
    /// <kbd>0</kbd> on a US keyboard.
    Digit0,
    /// <kbd>1</kbd> on a US keyboard.
    Digit1,
    /// <kbd>2</kbd> on a US keyboard.
    Digit2,
    /// <kbd>3</kbd> on a US keyboard.
    Digit3,
    /// <kbd>4</kbd> on a US keyboard.
    Digit4,
    /// <kbd>5</kbd> on a US keyboard.
    Digit5,
    /// <kbd>6</kbd> on a US keyboard.
    Digit6,
    /// <kbd>7</kbd> on a US keyboard.
    Digit7,
    /// <kbd>8</kbd> on a US keyboard.
    Digit8,
    /// <kbd>9</kbd> on a US keyboard.
    Digit9,
    /// <kbd>=</kbd> on a US keyboard.
    Equal,
    /// Located between the left <kbd>Shift</kbd> and <kbd>Z</kbd> keys.
    /// Labeled <kbd>\\</kbd> on a UK keyboard.
    IntlBackslash,
    /// Located between the <kbd>/</kbd> and right <kbd>Shift</kbd> keys.
    /// Labeled <kbd>\\</kbd> (ro) on a Japanese keyboard.
    IntlRo,
    /// Located between the <kbd>=</kbd> and <kbd>Backspace</kbd> keys.
    /// Labeled <kbd>¥</kbd> (yen) on a Japanese keyboard. <kbd>\\</kbd> on a
    /// Russian keyboard.
    IntlYen,
    /// <kbd>a</kbd> on a US keyboard.
    /// Labeled <kbd>q</kbd> on an AZERTY (e.g., French) keyboard.
    KeyA,
    /// <kbd>b</kbd> on a US keyboard.
    KeyB,
    /// <kbd>c</kbd> on a US keyboard.
    KeyC,
    /// <kbd>d</kbd> on a US keyboard.
    KeyD,
    /// <kbd>e</kbd> on a US keyboard.
    KeyE,
    /// <kbd>f</kbd> on a US keyboard.
    KeyF,
    /// <kbd>g</kbd> on a US keyboard.
    KeyG,
    /// <kbd>h</kbd> on a US keyboard.
    KeyH,
    /// <kbd>i</kbd> on a US keyboard.
    KeyI,
    /// <kbd>j</kbd> on a US keyboard.
    KeyJ,
    /// <kbd>k</kbd> on a US keyboard.
    KeyK,
    /// <kbd>l</kbd> on a US keyboard.
    KeyL,
    /// <kbd>m</kbd> on a US keyboard.
    KeyM,
    /// <kbd>n</kbd> on a US keyboard.
    KeyN,
    /// <kbd>o</kbd> on a US keyboard.
    KeyO,
    /// <kbd>p</kbd> on a US keyboard.
    KeyP,
    /// <kbd>q</kbd> on a US keyboard.
    /// Labeled <kbd>a</kbd> on an AZERTY (e.g., French) keyboard.
    KeyQ,
    /// <kbd>r</kbd> on a US keyboard.
    KeyR,
    /// <kbd>s</kbd> on a US keyboard.
    KeyS,
    /// <kbd>t</kbd> on a US keyboard.
    KeyT,
    /// <kbd>u</kbd> on a US keyboard.
    KeyU,
    /// <kbd>v</kbd> on a US keyboard.
    KeyV,
    /// <kbd>w</kbd> on a US keyboard.
    /// Labeled <kbd>z</kbd> on an AZERTY (e.g., French) keyboard.
    KeyW,
    /// <kbd>x</kbd> on a US keyboard.
    KeyX,
    /// <kbd>y</kbd> on a US keyboard.
    /// Labeled <kbd>z</kbd> on a QWERTZ (e.g., German) keyboard.
    KeyY,
    /// <kbd>z</kbd> on a US keyboard.
    /// Labeled <kbd>w</kbd> on an AZERTY (e.g., French) keyboard, and <kbd>y</kbd> on a
    /// QWERTZ (e.g., German) keyboard.
    KeyZ,
    /// <kbd>-</kbd> on a US keyboard.
    Minus,
    /// <kbd>.</kbd> on a US keyboard.
    Period,
    /// <kbd>'</kbd> on a US keyboard.
    Quote,
    /// <kbd>;</kbd> on a US keyboard.
    Semicolon,
    /// <kbd>/</kbd> on a US keyboard.
    Slash,
    /// <kbd>Alt</kbd>, <kbd>Option</kbd>, or <kbd>⌥</kbd>.
    AltLeft,
    /// <kbd>Alt</kbd>, <kbd>Option</kbd>, or <kbd>⌥</kbd>.
    /// This is labeled <kbd>AltGr</kbd> on many keyboard layouts.
    AltRight,
    /// <kbd>Backspace</kbd> or <kbd>⌫</kbd>.
    /// Labeled <kbd>Delete</kbd> on Apple keyboards.
    Backspace,
    /// <kbd>CapsLock</kbd> or <kbd>⇪</kbd>
    CapsLock,
    /// The application context menu key, which is typically found between the right
    /// <kbd>Super</kbd> key and the right <kbd>Control</kbd> key.
    ContextMenu,
    /// <kbd>Control</kbd> or <kbd>⌃</kbd>
    ControlLeft,
    /// <kbd>Control</kbd> or <kbd>⌃</kbd>
    ControlRight,
    /// <kbd>Enter</kbd> or <kbd>↵</kbd>. Labeled <kbd>Return</kbd> on Apple keyboards.
    Enter,
    /// The Windows, <kbd>⌘</kbd>, <kbd>Command</kbd>, or other OS symbol key.
    SuperLeft,
    /// The Windows, <kbd>⌘</kbd>, <kbd>Command</kbd>, or other OS symbol key.
    SuperRight,
    /// <kbd>Shift</kbd> or <kbd>⇧</kbd>
    ShiftLeft,
    /// <kbd>Shift</kbd> or <kbd>⇧</kbd>
    ShiftRight,
    /// <kbd> </kbd> (space)
    Space,
    /// <kbd>Tab</kbd> or <kbd>⇥</kbd>
    Tab,
    /// Japanese: <kbd>変</kbd> (henkan)
    Convert,
    /// Japanese: <kbd>カタカナ</kbd>/<kbd>ひらがな</kbd>/<kbd>ローマ字</kbd>
    /// (katakana/hiragana/romaji)
    KanaMode,
    /// Korean: HangulMode <kbd>한/영</kbd> (han/yeong)
    ///
    /// Japanese (Mac keyboard): <kbd>か</kbd> (kana)
    Lang1,
    /// Korean: Hanja <kbd>한</kbd> (hanja)
    ///
    /// Japanese (Mac keyboard): <kbd>英</kbd> (eisu)
    Lang2,
    /// Japanese (word-processing keyboard): Katakana
    Lang3,
    /// Japanese (word-processing keyboard): Hiragana
    Lang4,
    /// Japanese (word-processing keyboard): Zenkaku/Hankaku
    Lang5,
    /// Japanese: <kbd>無変換</kbd> (muhenkan)
    NonConvert,
    /// <kbd>⌦</kbd>. The forward delete key.
    /// Note that on Apple keyboards, the key labelled <kbd>Delete</kbd> on the main part of
    /// the keyboard is encoded as [`Backspace`].
    ///
    /// [`Backspace`]: Self::Backspace
    Delete,
    /// <kbd>Page Down</kbd>, <kbd>End</kbd>, or <kbd>↘</kbd>
    End,
    /// <kbd>Help</kbd>. Not present on standard PC keyboards.
    Help,
    /// <kbd>Home</kbd> or <kbd>↖</kbd>
    Home,
    /// <kbd>Insert</kbd> or <kbd>Ins</kbd>. Not present on Apple keyboards.
    Insert,
    /// <kbd>Page Down</kbd>, <kbd>PgDn</kbd>, or <kbd>⇟</kbd>
    PageDown,
    /// <kbd>Page Up</kbd>, <kbd>PgUp</kbd>, or <kbd>⇞</kbd>
    PageUp,
    /// <kbd>↓</kbd>
    ArrowDown,
    /// <kbd>←</kbd>
    ArrowLeft,
    /// <kbd>→</kbd>
    ArrowRight,
    /// <kbd>↑</kbd>
    ArrowUp,
    /// On the Mac, this is used for the numpad <kbd>Clear</kbd> key.
    NumLock,
    /// <kbd>0 Ins</kbd> on a keyboard. <kbd>0</kbd> on a phone or remote control
    Numpad0,
    /// <kbd>1 End</kbd> on a keyboard. <kbd>1</kbd> or <kbd>1 QZ</kbd> on a phone or remote
    /// control
    Numpad1,
    /// <kbd>2 ↓</kbd> on a keyboard. <kbd>2 ABC</kbd> on a phone or remote control
    Numpad2,
    /// <kbd>3 PgDn</kbd> on a keyboard. <kbd>3 DEF</kbd> on a phone or remote control
    Numpad3,
    /// <kbd>4 ←</kbd> on a keyboard. <kbd>4 GHI</kbd> on a phone or remote control
    Numpad4,
    /// <kbd>5</kbd> on a keyboard. <kbd>5 JKL</kbd> on a phone or remote control
    Numpad5,
    /// <kbd>6 →</kbd> on a keyboard. <kbd>6 MNO</kbd> on a phone or remote control
    Numpad6,
    /// <kbd>7 Home</kbd> on a keyboard. <kbd>7 PQRS</kbd> or <kbd>7 PRS</kbd> on a phone
    /// or remote control
    Numpad7,
    /// <kbd>8 ↑</kbd> on a keyboard. <kbd>8 TUV</kbd> on a phone or remote control
    Numpad8,
    /// <kbd>9 PgUp</kbd> on a keyboard. <kbd>9 WXYZ</kbd> or <kbd>9 WXY</kbd> on a phone
    /// or remote control
    Numpad9,
    /// <kbd>+</kbd>
    NumpadAdd,
    /// Found on the Microsoft Natural Keyboard.
    NumpadBackspace,
    /// <kbd>C</kbd> or <kbd>A</kbd> (All Clear). Also for use with numpads that have a
    /// <kbd>Clear</kbd> key that is separate from the <kbd>NumLock</kbd> key. On the Mac, the
    /// numpad <kbd>Clear</kbd> key is encoded as [`NumLock`].
    ///
    /// [`NumLock`]: Self::NumLock
    NumpadClear,
    /// <kbd>C</kbd> (Clear Entry)
    NumpadClearEntry,
    /// <kbd>,</kbd> (thousands separator). For locales where the thousands separator
    /// is a "." (e.g., Brazil), this key may generate a <kbd>.</kbd>.
    NumpadComma,
    /// <kbd>. Del</kbd>. For locales where the decimal separator is "," (e.g.,
    /// Brazil), this key may generate a <kbd>,</kbd>.
    NumpadDecimal,
    /// <kbd>/</kbd>
    NumpadDivide,
    NumpadEnter,
    /// <kbd>=</kbd>
    NumpadEqual,
    /// <kbd>#</kbd> on a phone or remote control device. This key is typically found
    /// below the <kbd>9</kbd> key and to the right of the <kbd>0</kbd> key.
    NumpadHash,
    /// <kbd>M</kbd> Add current entry to the value stored in memory.
    NumpadMemoryAdd,
    /// <kbd>M</kbd> Clear the value stored in memory.
    NumpadMemoryClear,
    /// <kbd>M</kbd> Replace the current entry with the value stored in memory.
    NumpadMemoryRecall,
    /// <kbd>M</kbd> Replace the value stored in memory with the current entry.
    NumpadMemoryStore,
    /// <kbd>M</kbd> Subtract current entry from the value stored in memory.
    NumpadMemorySubtract,
    /// <kbd>*</kbd> on a keyboard. For use with numpads that provide mathematical
    /// operations (<kbd>+</kbd>, <kbd>-</kbd> <kbd>*</kbd> and <kbd>/</kbd>).
    ///
    /// Use `NumpadStar` for the <kbd>*</kbd> key on phones and remote controls.
    NumpadMultiply,
    /// <kbd>(</kbd> Found on the Microsoft Natural Keyboard.
    NumpadParenLeft,
    /// <kbd>)</kbd> Found on the Microsoft Natural Keyboard.
    NumpadParenRight,
    /// <kbd>*</kbd> on a phone or remote control device.
    ///
    /// This key is typically found below the <kbd>7</kbd> key and to the left of
    /// the <kbd>0</kbd> key.
    ///
    /// Use <kbd>"NumpadMultiply"</kbd> for the <kbd>*</kbd> key on
    /// numeric keypads.
    NumpadStar,
    /// <kbd>-</kbd>
    NumpadSubtract,
    /// <kbd>Esc</kbd> or <kbd>⎋</kbd>
    Escape,
    /// <kbd>Fn</kbd> This is typically a hardware key that does not generate a separate code.
    Fn,
    /// <kbd>FLock</kbd> or <kbd>FnLock</kbd>. Function Lock key. Found on the Microsoft
    /// Natural Keyboard.
    FnLock,
    /// <kbd>PrtScr SysRq</kbd> or <kbd>Print Screen</kbd>
    PrintScreen,
    /// <kbd>Scroll Lock</kbd>
    ScrollLock,
    /// <kbd>Pause Break</kbd>
    Pause,
    /// Some laptops place this key to the left of the <kbd>↑</kbd> key.
    ///
    /// This also the "back" button (triangle) on Android.
    BrowserBack,
    BrowserFavorites,
    /// Some laptops place this key to the right of the <kbd>↑</kbd> key.
    BrowserForward,
    /// The "home" button on Android.
    BrowserHome,
    BrowserRefresh,
    BrowserSearch,
    BrowserStop,
    /// <kbd>Eject</kbd> or <kbd>⏏</kbd>. This key is placed in the function section on some Apple
    /// keyboards.
    Eject,
    /// Sometimes labelled <kbd>My Computer</kbd> on the keyboard
    LaunchApp1,
    /// Sometimes labelled <kbd>Calculator</kbd> on the keyboard
    LaunchApp2,
    LaunchMail,
    MediaPlayPause,
    MediaSelect,
    MediaStop,
    MediaTrackNext,
    MediaTrackPrevious,
    /// This key is placed in the function section on some Apple keyboards, replacing the
    /// <kbd>Eject</kbd> key.
    Power,
    Sleep,
    AudioVolumeDown,
    AudioVolumeMute,
    AudioVolumeUp,
    WakeUp,
    // Legacy modifier key. Also called "Super" in certain places.
    Meta,
    // Legacy modifier key.
    Hyper,
    Turbo,
    Abort,
    Resume,
    Suspend,
    /// Found on Sun’s USB keyboard.
    Again,
    /// Found on Sun’s USB keyboard.
    Copy,
    /// Found on Sun’s USB keyboard.
    Cut,
    /// Found on Sun’s USB keyboard.
    Find,
    /// Found on Sun’s USB keyboard.
    Open,
    /// Found on Sun’s USB keyboard.
    Paste,
    /// Found on Sun’s USB keyboard.
    Props,
    /// Found on Sun’s USB keyboard.
    Select,
    /// Found on Sun’s USB keyboard.
    Undo,
    /// Use for dedicated <kbd>ひらがな</kbd> key found on some Japanese word processing keyboards.
    Hiragana,
    /// Use for dedicated <kbd>カタカナ</kbd> key found on some Japanese word processing keyboards.
    Katakana,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F1,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F2,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F3,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F4,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F5,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F6,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F7,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F8,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F9,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F10,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F11,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F12,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F13,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F14,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F15,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F16,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F17,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F18,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F19,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F20,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F21,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F22,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F23,
    /// General-purpose function key.
    /// Usually found at the top of the keyboard.
    F24,
    /// General-purpose function key.
    F25,
    /// General-purpose function key.
    F26,
    /// General-purpose function key.
    F27,
    /// General-purpose function key.
    F28,
    /// General-purpose function key.
    F29,
    /// General-purpose function key.
    F30,
    /// General-purpose function key.
    F31,
    /// General-purpose function key.
    F32,
    /// General-purpose function key.
    F33,
    /// General-purpose function key.
    F34,
    /// General-purpose function key.
    F35,
}

impl KeyCode {
    pub fn from_winit_keycode(keycode: winit::keyboard::KeyCode) -> Option<Self> {
        match keycode {
            winit::keyboard::KeyCode::Backquote => Some(Self::Backquote),
            winit::keyboard::KeyCode::Backslash => Some(Self::Backslash),
            winit::keyboard::KeyCode::BracketLeft => Some(Self::BracketLeft),
            winit::keyboard::KeyCode::BracketRight => Some(Self::BracketRight),
            winit::keyboard::KeyCode::Comma => Some(Self::Comma),
            winit::keyboard::KeyCode::Digit0 => Some(Self::Digit0),
            winit::keyboard::KeyCode::Digit1 => Some(Self::Digit1),
            winit::keyboard::KeyCode::Digit2 => Some(Self::Digit2),
            winit::keyboard::KeyCode::Digit3 => Some(Self::Digit3),
            winit::keyboard::KeyCode::Digit4 => Some(Self::Digit4),
            winit::keyboard::KeyCode::Digit5 => Some(Self::Digit5),
            winit::keyboard::KeyCode::Digit6 => Some(Self::Digit6),
            winit::keyboard::KeyCode::Digit7 => Some(Self::Digit7),
            winit::keyboard::KeyCode::Digit8 => Some(Self::Digit8),
            winit::keyboard::KeyCode::Digit9 => Some(Self::Digit9),
            winit::keyboard::KeyCode::Equal => Some(Self::Equal),
            winit::keyboard::KeyCode::IntlBackslash => Some(Self::IntlBackslash),
            winit::keyboard::KeyCode::IntlRo => Some(Self::IntlRo),
            winit::keyboard::KeyCode::IntlYen => Some(Self::IntlYen),
            winit::keyboard::KeyCode::KeyA => Some(Self::KeyA),
            winit::keyboard::KeyCode::KeyB => Some(Self::KeyB),
            winit::keyboard::KeyCode::KeyC => Some(Self::KeyC),
            winit::keyboard::KeyCode::KeyD => Some(Self::KeyD),
            winit::keyboard::KeyCode::KeyE => Some(Self::KeyE),
            winit::keyboard::KeyCode::KeyF => Some(Self::KeyF),
            winit::keyboard::KeyCode::KeyG => Some(Self::KeyG),
            winit::keyboard::KeyCode::KeyH => Some(Self::KeyH),
            winit::keyboard::KeyCode::KeyI => Some(Self::KeyI),
            winit::keyboard::KeyCode::KeyJ => Some(Self::KeyJ),
            winit::keyboard::KeyCode::KeyK => Some(Self::KeyK),
            winit::keyboard::KeyCode::KeyL => Some(Self::KeyL),
            winit::keyboard::KeyCode::KeyM => Some(Self::KeyM),
            winit::keyboard::KeyCode::KeyN => Some(Self::KeyN),
            winit::keyboard::KeyCode::KeyO => Some(Self::KeyO),
            winit::keyboard::KeyCode::KeyP => Some(Self::KeyP),
            winit::keyboard::KeyCode::KeyQ => Some(Self::KeyQ),
            winit::keyboard::KeyCode::KeyR => Some(Self::KeyR),
            winit::keyboard::KeyCode::KeyS => Some(Self::KeyS),
            winit::keyboard::KeyCode::KeyT => Some(Self::KeyT),
            winit::keyboard::KeyCode::KeyU => Some(Self::KeyU),
            winit::keyboard::KeyCode::KeyV => Some(Self::KeyV),
            winit::keyboard::KeyCode::KeyW => Some(Self::KeyW),
            winit::keyboard::KeyCode::KeyX => Some(Self::KeyX),
            winit::keyboard::KeyCode::KeyY => Some(Self::KeyY),
            winit::keyboard::KeyCode::KeyZ => Some(Self::KeyZ),
            winit::keyboard::KeyCode::Minus => Some(Self::Minus),
            winit::keyboard::KeyCode::Period => Some(Self::Period),
            winit::keyboard::KeyCode::Quote => Some(Self::Quote),
            winit::keyboard::KeyCode::Semicolon => Some(Self::Semicolon),
            winit::keyboard::KeyCode::Slash => Some(Self::Slash),
            winit::keyboard::KeyCode::AltLeft => Some(Self::AltLeft),
            winit::keyboard::KeyCode::AltRight => Some(Self::AltRight),
            winit::keyboard::KeyCode::Backspace => Some(Self::Backspace),
            winit::keyboard::KeyCode::CapsLock => Some(Self::CapsLock),
            winit::keyboard::KeyCode::ContextMenu => Some(Self::ContextMenu),
            winit::keyboard::KeyCode::ControlLeft => Some(Self::ControlLeft),
            winit::keyboard::KeyCode::ControlRight => Some(Self::ControlRight),
            winit::keyboard::KeyCode::Enter => Some(Self::Enter),
            winit::keyboard::KeyCode::SuperLeft => Some(Self::SuperLeft),
            winit::keyboard::KeyCode::SuperRight => Some(Self::SuperRight),
            winit::keyboard::KeyCode::ShiftLeft => Some(Self::ShiftLeft),
            winit::keyboard::KeyCode::ShiftRight => Some(Self::ShiftRight),
            winit::keyboard::KeyCode::Space => Some(Self::Space),
            winit::keyboard::KeyCode::Tab => Some(Self::Tab),
            winit::keyboard::KeyCode::Convert => Some(Self::Convert),
            winit::keyboard::KeyCode::KanaMode => Some(Self::KanaMode),
            winit::keyboard::KeyCode::Lang1 => Some(Self::Lang1),
            winit::keyboard::KeyCode::Lang2 => Some(Self::Lang2),
            winit::keyboard::KeyCode::Lang3 => Some(Self::Lang3),
            winit::keyboard::KeyCode::Lang4 => Some(Self::Lang4),
            winit::keyboard::KeyCode::Lang5 => Some(Self::Lang5),
            winit::keyboard::KeyCode::NonConvert => Some(Self::NonConvert),
            winit::keyboard::KeyCode::Delete => Some(Self::Delete),
            winit::keyboard::KeyCode::End => Some(Self::End),
            winit::keyboard::KeyCode::Help => Some(Self::Help),
            winit::keyboard::KeyCode::Home => Some(Self::Home),
            winit::keyboard::KeyCode::Insert => Some(Self::Insert),
            winit::keyboard::KeyCode::PageDown => Some(Self::PageDown),
            winit::keyboard::KeyCode::PageUp => Some(Self::PageUp),
            winit::keyboard::KeyCode::ArrowDown => Some(Self::ArrowDown),
            winit::keyboard::KeyCode::ArrowLeft => Some(Self::ArrowLeft),
            winit::keyboard::KeyCode::ArrowRight => Some(Self::ArrowRight),
            winit::keyboard::KeyCode::ArrowUp => Some(Self::ArrowUp),
            winit::keyboard::KeyCode::NumLock => Some(Self::NumLock),
            winit::keyboard::KeyCode::Numpad0 => Some(Self::Numpad0),
            winit::keyboard::KeyCode::Numpad1 => Some(Self::Numpad1),
            winit::keyboard::KeyCode::Numpad2 => Some(Self::Numpad2),
            winit::keyboard::KeyCode::Numpad3 => Some(Self::Numpad3),
            winit::keyboard::KeyCode::Numpad4 => Some(Self::Numpad4),
            winit::keyboard::KeyCode::Numpad5 => Some(Self::Numpad5),
            winit::keyboard::KeyCode::Numpad6 => Some(Self::Numpad6),
            winit::keyboard::KeyCode::Numpad7 => Some(Self::Numpad7),
            winit::keyboard::KeyCode::Numpad8 => Some(Self::Numpad8),
            winit::keyboard::KeyCode::Numpad9 => Some(Self::Numpad9),
            winit::keyboard::KeyCode::NumpadAdd => Some(Self::NumpadAdd),
            winit::keyboard::KeyCode::NumpadBackspace => Some(Self::NumpadBackspace),
            winit::keyboard::KeyCode::NumpadClear => Some(Self::NumpadClear),
            winit::keyboard::KeyCode::NumpadClearEntry => Some(Self::NumpadClearEntry),
            winit::keyboard::KeyCode::NumpadComma => Some(Self::NumpadComma),
            winit::keyboard::KeyCode::NumpadDecimal => Some(Self::NumpadDecimal),
            winit::keyboard::KeyCode::NumpadDivide => Some(Self::NumpadDivide),
            winit::keyboard::KeyCode::NumpadEnter => Some(Self::NumpadEnter),
            winit::keyboard::KeyCode::NumpadEqual => Some(Self::NumpadEqual),
            winit::keyboard::KeyCode::NumpadHash => Some(Self::NumpadHash),
            winit::keyboard::KeyCode::NumpadMemoryAdd => Some(Self::NumpadMemoryAdd),
            winit::keyboard::KeyCode::NumpadMemoryClear => Some(Self::NumpadMemoryClear),
            winit::keyboard::KeyCode::NumpadMemoryRecall => Some(Self::NumpadMemoryRecall),
            winit::keyboard::KeyCode::NumpadMemoryStore => Some(Self::NumpadMemoryStore),
            winit::keyboard::KeyCode::NumpadMemorySubtract => Some(Self::NumpadMemorySubtract),
            winit::keyboard::KeyCode::NumpadMultiply => Some(Self::NumpadMultiply),
            winit::keyboard::KeyCode::NumpadParenLeft => Some(Self::NumpadParenLeft),
            winit::keyboard::KeyCode::NumpadParenRight => Some(Self::NumpadParenRight),
            winit::keyboard::KeyCode::NumpadStar => Some(Self::NumpadStar),
            winit::keyboard::KeyCode::NumpadSubtract => Some(Self::NumpadSubtract),
            winit::keyboard::KeyCode::Escape => Some(Self::Escape),
            winit::keyboard::KeyCode::Fn => Some(Self::Fn),
            winit::keyboard::KeyCode::FnLock => Some(Self::FnLock),
            winit::keyboard::KeyCode::PrintScreen => Some(Self::PrintScreen),
            winit::keyboard::KeyCode::ScrollLock => Some(Self::ScrollLock),
            winit::keyboard::KeyCode::Pause => Some(Self::Pause),
            winit::keyboard::KeyCode::BrowserBack => Some(Self::BrowserBack),
            winit::keyboard::KeyCode::BrowserFavorites => Some(Self::BrowserFavorites),
            winit::keyboard::KeyCode::BrowserForward => Some(Self::BrowserForward),
            winit::keyboard::KeyCode::BrowserHome => Some(Self::BrowserHome),
            winit::keyboard::KeyCode::BrowserRefresh => Some(Self::BrowserRefresh),
            winit::keyboard::KeyCode::BrowserSearch => Some(Self::BrowserSearch),
            winit::keyboard::KeyCode::BrowserStop => Some(Self::BrowserStop),
            winit::keyboard::KeyCode::Eject => Some(Self::Eject),
            winit::keyboard::KeyCode::LaunchApp1 => Some(Self::LaunchApp1),
            winit::keyboard::KeyCode::LaunchApp2 => Some(Self::LaunchApp2),
            winit::keyboard::KeyCode::LaunchMail => Some(Self::LaunchMail),
            winit::keyboard::KeyCode::MediaPlayPause => Some(Self::MediaPlayPause),
            winit::keyboard::KeyCode::MediaSelect => Some(Self::MediaSelect),
            winit::keyboard::KeyCode::MediaStop => Some(Self::MediaStop),
            winit::keyboard::KeyCode::MediaTrackNext => Some(Self::MediaTrackNext),
            winit::keyboard::KeyCode::MediaTrackPrevious => Some(Self::MediaTrackPrevious),
            winit::keyboard::KeyCode::Power => Some(Self::Power),
            winit::keyboard::KeyCode::Sleep => Some(Self::Sleep),
            winit::keyboard::KeyCode::AudioVolumeDown => Some(Self::AudioVolumeDown),
            winit::keyboard::KeyCode::AudioVolumeMute => Some(Self::AudioVolumeMute),
            winit::keyboard::KeyCode::AudioVolumeUp => Some(Self::AudioVolumeUp),
            winit::keyboard::KeyCode::WakeUp => Some(Self::WakeUp),
            winit::keyboard::KeyCode::Meta => Some(Self::Meta),
            winit::keyboard::KeyCode::Hyper => Some(Self::Hyper),
            winit::keyboard::KeyCode::Turbo => Some(Self::Turbo),
            winit::keyboard::KeyCode::Abort => Some(Self::Abort),
            winit::keyboard::KeyCode::Resume => Some(Self::Resume),
            winit::keyboard::KeyCode::Suspend => Some(Self::Suspend),
            winit::keyboard::KeyCode::Again => Some(Self::Again),
            winit::keyboard::KeyCode::Copy => Some(Self::Copy),
            winit::keyboard::KeyCode::Cut => Some(Self::Cut),
            winit::keyboard::KeyCode::Find => Some(Self::Find),
            winit::keyboard::KeyCode::Open => Some(Self::Open),
            winit::keyboard::KeyCode::Paste => Some(Self::Paste),
            winit::keyboard::KeyCode::Props => Some(Self::Props),
            winit::keyboard::KeyCode::Select => Some(Self::Select),
            winit::keyboard::KeyCode::Undo => Some(Self::Undo),
            winit::keyboard::KeyCode::Hiragana => Some(Self::Hiragana),
            winit::keyboard::KeyCode::Katakana => Some(Self::Katakana),
            winit::keyboard::KeyCode::F1 => Some(Self::F1),
            winit::keyboard::KeyCode::F2 => Some(Self::F2),
            winit::keyboard::KeyCode::F3 => Some(Self::F3),
            winit::keyboard::KeyCode::F4 => Some(Self::F4),
            winit::keyboard::KeyCode::F5 => Some(Self::F5),
            winit::keyboard::KeyCode::F6 => Some(Self::F6),
            winit::keyboard::KeyCode::F7 => Some(Self::F7),
            winit::keyboard::KeyCode::F8 => Some(Self::F8),
            winit::keyboard::KeyCode::F9 => Some(Self::F9),
            winit::keyboard::KeyCode::F10 => Some(Self::F10),
            winit::keyboard::KeyCode::F11 => Some(Self::F11),
            winit::keyboard::KeyCode::F12 => Some(Self::F12),
            winit::keyboard::KeyCode::F13 => Some(Self::F13),
            winit::keyboard::KeyCode::F14 => Some(Self::F14),
            winit::keyboard::KeyCode::F15 => Some(Self::F15),
            winit::keyboard::KeyCode::F16 => Some(Self::F16),
            winit::keyboard::KeyCode::F17 => Some(Self::F17),
            winit::keyboard::KeyCode::F18 => Some(Self::F18),
            winit::keyboard::KeyCode::F19 => Some(Self::F19),
            winit::keyboard::KeyCode::F20 => Some(Self::F20),
            winit::keyboard::KeyCode::F21 => Some(Self::F21),
            winit::keyboard::KeyCode::F22 => Some(Self::F22),
            winit::keyboard::KeyCode::F23 => Some(Self::F23),
            winit::keyboard::KeyCode::F24 => Some(Self::F24),
            winit::keyboard::KeyCode::F25 => Some(Self::F25),
            winit::keyboard::KeyCode::F26 => Some(Self::F26),
            winit::keyboard::KeyCode::F27 => Some(Self::F27),
            winit::keyboard::KeyCode::F28 => Some(Self::F28),
            winit::keyboard::KeyCode::F29 => Some(Self::F29),
            winit::keyboard::KeyCode::F30 => Some(Self::F30),
            winit::keyboard::KeyCode::F31 => Some(Self::F31),
            winit::keyboard::KeyCode::F32 => Some(Self::F32),
            winit::keyboard::KeyCode::F33 => Some(Self::F33),
            winit::keyboard::KeyCode::F34 => Some(Self::F34),
            winit::keyboard::KeyCode::F35 => Some(Self::F35),
            _ => None,
        }
    }

    pub fn as_winit_keycode(&self) -> Option<winit::keyboard::KeyCode> {
        match self {
            KeyCode::Backquote => Some(winit::keyboard::KeyCode::Backquote),
            KeyCode::Backslash => Some(winit::keyboard::KeyCode::Backslash),
            KeyCode::BracketLeft => Some(winit::keyboard::KeyCode::BracketLeft),
            KeyCode::BracketRight => Some(winit::keyboard::KeyCode::BracketRight),
            KeyCode::Comma => Some(winit::keyboard::KeyCode::Comma),
            KeyCode::Digit0 => Some(winit::keyboard::KeyCode::Digit0),
            KeyCode::Digit1 => Some(winit::keyboard::KeyCode::Digit1),
            KeyCode::Digit2 => Some(winit::keyboard::KeyCode::Digit2),
            KeyCode::Digit3 => Some(winit::keyboard::KeyCode::Digit3),
            KeyCode::Digit4 => Some(winit::keyboard::KeyCode::Digit4),
            KeyCode::Digit5 => Some(winit::keyboard::KeyCode::Digit5),
            KeyCode::Digit6 => Some(winit::keyboard::KeyCode::Digit6),
            KeyCode::Digit7 => Some(winit::keyboard::KeyCode::Digit7),
            KeyCode::Digit8 => Some(winit::keyboard::KeyCode::Digit8),
            KeyCode::Digit9 => Some(winit::keyboard::KeyCode::Digit9),
            KeyCode::Equal => Some(winit::keyboard::KeyCode::Equal),
            KeyCode::IntlBackslash => Some(winit::keyboard::KeyCode::IntlBackslash),
            KeyCode::IntlRo => Some(winit::keyboard::KeyCode::IntlRo),
            KeyCode::IntlYen => Some(winit::keyboard::KeyCode::IntlYen),
            KeyCode::KeyA => Some(winit::keyboard::KeyCode::KeyA),
            KeyCode::KeyB => Some(winit::keyboard::KeyCode::KeyB),
            KeyCode::KeyC => Some(winit::keyboard::KeyCode::KeyC),
            KeyCode::KeyD => Some(winit::keyboard::KeyCode::KeyD),
            KeyCode::KeyE => Some(winit::keyboard::KeyCode::KeyE),
            KeyCode::KeyF => Some(winit::keyboard::KeyCode::KeyF),
            KeyCode::KeyG => Some(winit::keyboard::KeyCode::KeyG),
            KeyCode::KeyH => Some(winit::keyboard::KeyCode::KeyH),
            KeyCode::KeyI => Some(winit::keyboard::KeyCode::KeyI),
            KeyCode::KeyJ => Some(winit::keyboard::KeyCode::KeyJ),
            KeyCode::KeyK => Some(winit::keyboard::KeyCode::KeyK),
            KeyCode::KeyL => Some(winit::keyboard::KeyCode::KeyL),
            KeyCode::KeyM => Some(winit::keyboard::KeyCode::KeyM),
            KeyCode::KeyN => Some(winit::keyboard::KeyCode::KeyN),
            KeyCode::KeyO => Some(winit::keyboard::KeyCode::KeyO),
            KeyCode::KeyP => Some(winit::keyboard::KeyCode::KeyP),
            KeyCode::KeyQ => Some(winit::keyboard::KeyCode::KeyQ),
            KeyCode::KeyR => Some(winit::keyboard::KeyCode::KeyR),
            KeyCode::KeyS => Some(winit::keyboard::KeyCode::KeyS),
            KeyCode::KeyT => Some(winit::keyboard::KeyCode::KeyT),
            KeyCode::KeyU => Some(winit::keyboard::KeyCode::KeyU),
            KeyCode::KeyV => Some(winit::keyboard::KeyCode::KeyV),
            KeyCode::KeyW => Some(winit::keyboard::KeyCode::KeyW),
            KeyCode::KeyX => Some(winit::keyboard::KeyCode::KeyX),
            KeyCode::KeyY => Some(winit::keyboard::KeyCode::KeyY),
            KeyCode::KeyZ => Some(winit::keyboard::KeyCode::KeyZ),
            KeyCode::Minus => Some(winit::keyboard::KeyCode::Minus),
            KeyCode::Period => Some(winit::keyboard::KeyCode::Period),
            KeyCode::Quote => Some(winit::keyboard::KeyCode::Quote),
            KeyCode::Semicolon => Some(winit::keyboard::KeyCode::Semicolon),
            KeyCode::Slash => Some(winit::keyboard::KeyCode::Slash),
            KeyCode::AltLeft => Some(winit::keyboard::KeyCode::AltLeft),
            KeyCode::AltRight => Some(winit::keyboard::KeyCode::AltRight),
            KeyCode::Backspace => Some(winit::keyboard::KeyCode::Backspace),
            KeyCode::CapsLock => Some(winit::keyboard::KeyCode::CapsLock),
            KeyCode::ContextMenu => Some(winit::keyboard::KeyCode::ContextMenu),
            KeyCode::ControlLeft => Some(winit::keyboard::KeyCode::ControlLeft),
            KeyCode::ControlRight => Some(winit::keyboard::KeyCode::ControlRight),
            KeyCode::Enter => Some(winit::keyboard::KeyCode::Enter),
            KeyCode::SuperLeft => Some(winit::keyboard::KeyCode::SuperLeft),
            KeyCode::SuperRight => Some(winit::keyboard::KeyCode::SuperRight),
            KeyCode::ShiftLeft => Some(winit::keyboard::KeyCode::ShiftLeft),
            KeyCode::ShiftRight => Some(winit::keyboard::KeyCode::ShiftRight),
            KeyCode::Space => Some(winit::keyboard::KeyCode::Space),
            KeyCode::Tab => Some(winit::keyboard::KeyCode::Tab),
            KeyCode::Convert => Some(winit::keyboard::KeyCode::Convert),
            KeyCode::KanaMode => Some(winit::keyboard::KeyCode::KanaMode),
            KeyCode::Lang1 => Some(winit::keyboard::KeyCode::Lang1),
            KeyCode::Lang2 => Some(winit::keyboard::KeyCode::Lang2),
            KeyCode::Lang3 => Some(winit::keyboard::KeyCode::Lang3),
            KeyCode::Lang4 => Some(winit::keyboard::KeyCode::Lang4),
            KeyCode::Lang5 => Some(winit::keyboard::KeyCode::Lang5),
            KeyCode::NonConvert => Some(winit::keyboard::KeyCode::NonConvert),
            KeyCode::Delete => Some(winit::keyboard::KeyCode::Delete),
            KeyCode::End => Some(winit::keyboard::KeyCode::End),
            KeyCode::Help => Some(winit::keyboard::KeyCode::Help),
            KeyCode::Home => Some(winit::keyboard::KeyCode::Home),
            KeyCode::Insert => Some(winit::keyboard::KeyCode::Insert),
            KeyCode::PageDown => Some(winit::keyboard::KeyCode::PageDown),
            KeyCode::PageUp => Some(winit::keyboard::KeyCode::PageUp),
            KeyCode::ArrowDown => Some(winit::keyboard::KeyCode::ArrowDown),
            KeyCode::ArrowLeft => Some(winit::keyboard::KeyCode::ArrowLeft),
            KeyCode::ArrowRight => Some(winit::keyboard::KeyCode::ArrowRight),
            KeyCode::ArrowUp => Some(winit::keyboard::KeyCode::ArrowUp),
            KeyCode::NumLock => Some(winit::keyboard::KeyCode::NumLock),
            KeyCode::Numpad0 => Some(winit::keyboard::KeyCode::Numpad0),
            KeyCode::Numpad1 => Some(winit::keyboard::KeyCode::Numpad1),
            KeyCode::Numpad2 => Some(winit::keyboard::KeyCode::Numpad2),
            KeyCode::Numpad3 => Some(winit::keyboard::KeyCode::Numpad3),
            KeyCode::Numpad4 => Some(winit::keyboard::KeyCode::Numpad4),
            KeyCode::Numpad5 => Some(winit::keyboard::KeyCode::Numpad5),
            KeyCode::Numpad6 => Some(winit::keyboard::KeyCode::Numpad6),
            KeyCode::Numpad7 => Some(winit::keyboard::KeyCode::Numpad7),
            KeyCode::Numpad8 => Some(winit::keyboard::KeyCode::Numpad8),
            KeyCode::Numpad9 => Some(winit::keyboard::KeyCode::Numpad9),
            KeyCode::NumpadAdd => Some(winit::keyboard::KeyCode::NumpadAdd),
            KeyCode::NumpadBackspace => Some(winit::keyboard::KeyCode::NumpadBackspace),
            KeyCode::NumpadClear => Some(winit::keyboard::KeyCode::NumpadClear),
            KeyCode::NumpadClearEntry => Some(winit::keyboard::KeyCode::NumpadClearEntry),
            KeyCode::NumpadComma => Some(winit::keyboard::KeyCode::NumpadComma),
            KeyCode::NumpadDecimal => Some(winit::keyboard::KeyCode::NumpadDecimal),
            KeyCode::NumpadDivide => Some(winit::keyboard::KeyCode::NumpadDivide),
            KeyCode::NumpadEnter => Some(winit::keyboard::KeyCode::NumpadEnter),
            KeyCode::NumpadEqual => Some(winit::keyboard::KeyCode::NumpadEqual),
            KeyCode::NumpadHash => Some(winit::keyboard::KeyCode::NumpadHash),
            KeyCode::NumpadMemoryAdd => Some(winit::keyboard::KeyCode::NumpadMemoryAdd),
            KeyCode::NumpadMemoryClear => Some(winit::keyboard::KeyCode::NumpadMemoryClear),
            KeyCode::NumpadMemoryRecall => Some(winit::keyboard::KeyCode::NumpadMemoryRecall),
            KeyCode::NumpadMemoryStore => Some(winit::keyboard::KeyCode::NumpadMemoryStore),
            KeyCode::NumpadMemorySubtract => Some(winit::keyboard::KeyCode::NumpadMemorySubtract),
            KeyCode::NumpadMultiply => Some(winit::keyboard::KeyCode::NumpadMultiply),
            KeyCode::NumpadParenLeft => Some(winit::keyboard::KeyCode::NumpadParenLeft),
            KeyCode::NumpadParenRight => Some(winit::keyboard::KeyCode::NumpadParenRight),
            KeyCode::NumpadStar => Some(winit::keyboard::KeyCode::NumpadStar),
            KeyCode::NumpadSubtract => Some(winit::keyboard::KeyCode::NumpadSubtract),
            KeyCode::Escape => Some(winit::keyboard::KeyCode::Escape),
            KeyCode::Fn => Some(winit::keyboard::KeyCode::Fn),
            KeyCode::FnLock => Some(winit::keyboard::KeyCode::FnLock),
            KeyCode::PrintScreen => Some(winit::keyboard::KeyCode::PrintScreen),
            KeyCode::ScrollLock => Some(winit::keyboard::KeyCode::ScrollLock),
            KeyCode::Pause => Some(winit::keyboard::KeyCode::Pause),
            KeyCode::BrowserBack => Some(winit::keyboard::KeyCode::BrowserBack),
            KeyCode::BrowserFavorites => Some(winit::keyboard::KeyCode::BrowserFavorites),
            KeyCode::BrowserForward => Some(winit::keyboard::KeyCode::BrowserForward),
            KeyCode::BrowserHome => Some(winit::keyboard::KeyCode::BrowserHome),
            KeyCode::BrowserRefresh => Some(winit::keyboard::KeyCode::BrowserRefresh),
            KeyCode::BrowserSearch => Some(winit::keyboard::KeyCode::BrowserSearch),
            KeyCode::BrowserStop => Some(winit::keyboard::KeyCode::BrowserStop),
            KeyCode::Eject => Some(winit::keyboard::KeyCode::Eject),
            KeyCode::LaunchApp1 => Some(winit::keyboard::KeyCode::LaunchApp1),
            KeyCode::LaunchApp2 => Some(winit::keyboard::KeyCode::LaunchApp2),
            KeyCode::LaunchMail => Some(winit::keyboard::KeyCode::LaunchMail),
            KeyCode::MediaPlayPause => Some(winit::keyboard::KeyCode::MediaPlayPause),
            KeyCode::MediaSelect => Some(winit::keyboard::KeyCode::MediaSelect),
            KeyCode::MediaStop => Some(winit::keyboard::KeyCode::MediaStop),
            KeyCode::MediaTrackNext => Some(winit::keyboard::KeyCode::MediaTrackNext),
            KeyCode::MediaTrackPrevious => Some(winit::keyboard::KeyCode::MediaTrackPrevious),
            KeyCode::Power => Some(winit::keyboard::KeyCode::Power),
            KeyCode::Sleep => Some(winit::keyboard::KeyCode::Sleep),
            KeyCode::AudioVolumeDown => Some(winit::keyboard::KeyCode::AudioVolumeDown),
            KeyCode::AudioVolumeMute => Some(winit::keyboard::KeyCode::AudioVolumeMute),
            KeyCode::AudioVolumeUp => Some(winit::keyboard::KeyCode::AudioVolumeUp),
            KeyCode::WakeUp => Some(winit::keyboard::KeyCode::WakeUp),
            KeyCode::Meta => Some(winit::keyboard::KeyCode::Meta),
            KeyCode::Hyper => Some(winit::keyboard::KeyCode::Hyper),
            KeyCode::Turbo => Some(winit::keyboard::KeyCode::Turbo),
            KeyCode::Abort => Some(winit::keyboard::KeyCode::Abort),
            KeyCode::Resume => Some(winit::keyboard::KeyCode::Resume),
            KeyCode::Suspend => Some(winit::keyboard::KeyCode::Suspend),
            KeyCode::Again => Some(winit::keyboard::KeyCode::Again),
            KeyCode::Copy => Some(winit::keyboard::KeyCode::Copy),
            KeyCode::Cut => Some(winit::keyboard::KeyCode::Cut),
            KeyCode::Find => Some(winit::keyboard::KeyCode::Find),
            KeyCode::Open => Some(winit::keyboard::KeyCode::Open),
            KeyCode::Paste => Some(winit::keyboard::KeyCode::Paste),
            KeyCode::Props => Some(winit::keyboard::KeyCode::Props),
            KeyCode::Select => Some(winit::keyboard::KeyCode::Select),
            KeyCode::Undo => Some(winit::keyboard::KeyCode::Undo),
            KeyCode::Hiragana => Some(winit::keyboard::KeyCode::Hiragana),
            KeyCode::Katakana => Some(winit::keyboard::KeyCode::Katakana),
            KeyCode::F1 => Some(winit::keyboard::KeyCode::F1),
            KeyCode::F2 => Some(winit::keyboard::KeyCode::F2),
            KeyCode::F3 => Some(winit::keyboard::KeyCode::F3),
            KeyCode::F4 => Some(winit::keyboard::KeyCode::F4),
            KeyCode::F5 => Some(winit::keyboard::KeyCode::F5),
            KeyCode::F6 => Some(winit::keyboard::KeyCode::F6),
            KeyCode::F7 => Some(winit::keyboard::KeyCode::F7),
            KeyCode::F8 => Some(winit::keyboard::KeyCode::F8),
            KeyCode::F9 => Some(winit::keyboard::KeyCode::F9),
            KeyCode::F10 => Some(winit::keyboard::KeyCode::F10),
            KeyCode::F11 => Some(winit::keyboard::KeyCode::F11),
            KeyCode::F12 => Some(winit::keyboard::KeyCode::F12),
            KeyCode::F13 => Some(winit::keyboard::KeyCode::F13),
            KeyCode::F14 => Some(winit::keyboard::KeyCode::F14),
            KeyCode::F15 => Some(winit::keyboard::KeyCode::F15),
            KeyCode::F16 => Some(winit::keyboard::KeyCode::F16),
            KeyCode::F17 => Some(winit::keyboard::KeyCode::F17),
            KeyCode::F18 => Some(winit::keyboard::KeyCode::F18),
            KeyCode::F19 => Some(winit::keyboard::KeyCode::F19),
            KeyCode::F20 => Some(winit::keyboard::KeyCode::F20),
            KeyCode::F21 => Some(winit::keyboard::KeyCode::F21),
            KeyCode::F22 => Some(winit::keyboard::KeyCode::F22),
            KeyCode::F23 => Some(winit::keyboard::KeyCode::F23),
            KeyCode::F24 => Some(winit::keyboard::KeyCode::F24),
            KeyCode::F25 => Some(winit::keyboard::KeyCode::F25),
            KeyCode::F26 => Some(winit::keyboard::KeyCode::F26),
            KeyCode::F27 => Some(winit::keyboard::KeyCode::F27),
            KeyCode::F28 => Some(winit::keyboard::KeyCode::F28),
            KeyCode::F29 => Some(winit::keyboard::KeyCode::F29),
            KeyCode::F30 => Some(winit::keyboard::KeyCode::F30),
            KeyCode::F31 => Some(winit::keyboard::KeyCode::F31),
            KeyCode::F32 => Some(winit::keyboard::KeyCode::F32),
            KeyCode::F33 => Some(winit::keyboard::KeyCode::F33),
            KeyCode::F34 => Some(winit::keyboard::KeyCode::F34),
            KeyCode::F35 => Some(winit::keyboard::KeyCode::F35),
        }
    }
}