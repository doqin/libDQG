use crate::renderer::DrawPass;
use crate::types::Color;

pub const TITLEBAR_HEIGHT: f32 = 32.0;
const BUTTON_WIDTH: f32 = 46.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleBarButton {
    Minimize,
    Maximize,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleBarHit {
    None,
    Drag,
    Button(TitleBarButton),
}

pub struct TitleBar {
    pub hovered: Option<TitleBarButton>,
    resizable: bool,
}

impl TitleBar {
    pub fn new(resizable: bool) -> Self {
        Self { hovered: None, resizable }
    }

    fn button_rect(button: TitleBarButton, screen_w: f32) -> (f32, f32, f32, f32) {
        let index_from_right = match button {
            TitleBarButton::Close => 0.0,
            TitleBarButton::Maximize => 1.0,
            TitleBarButton::Minimize => 2.0,
        };
        let x = screen_w - BUTTON_WIDTH * (index_from_right + 1.0);
        (x, 0.0, BUTTON_WIDTH, TITLEBAR_HEIGHT)
    }

    pub fn hit_test(&self, x: f32, y: f32, screen_w: f32) -> TitleBarHit {
        if y < 0.0 || y > TITLEBAR_HEIGHT {
            return TitleBarHit::None;
        }
        for &button in &[TitleBarButton::Close, TitleBarButton::Maximize, TitleBarButton::Minimize] {
            if button == TitleBarButton::Maximize && !self.resizable {
                continue;
            }
            let (bx, by, bw, bh) = Self::button_rect(button, screen_w);
            if x >= bx && x < bx + bw && y >= by && y < by + bh {
                return TitleBarHit::Button(button);
            }
        }
        TitleBarHit::Drag
    }

    pub fn update_hover(&mut self, x: f32, y: f32, screen_w: f32) {
        self.hovered = match self.hit_test(x, y, screen_w) {
            TitleBarHit::Button(button) => Some(button),
            _ => None,
        };
    }

    pub fn render(&self, pass: &mut DrawPass, screen_w: u32, maximized: bool) {
        let w = screen_w as f32;

        for &button in &[TitleBarButton::Minimize, TitleBarButton::Maximize, TitleBarButton::Close] {
            let (bx, by, bw, bh) = Self::button_rect(button, w);

            if self.hovered == Some(button) {
                let bg = if button == TitleBarButton::Close {
                    Color::from_hex(0xff0000)
                } else {
                    Color::new(1.0, 1.0, 1.0, 0.15)
                };
                pass.draw_rect(bx, by, bw, bh, 0.0, bg);
            }

            let cx = bx + bw * 0.5;
            let cy = by + bh * 0.5;
            let icon_color = if button == TitleBarButton::Maximize && !self.resizable {
                Color::new(1.0, 1.0, 1.0, 0.35)
            } else {
                Color::new(1.0, 1.0, 1.0, 1.0)
            };

            match button {
                TitleBarButton::Minimize => {
                    pass.draw_line(cx - 5.0, cy, cx + 5.0, cy, 1.0, icon_color);
                }
                TitleBarButton::Maximize => {
                    if maximized {
                        pass.draw_rect(cx - 4.0, cy - 5.0, 8.0, 8.0, 1.0, icon_color);
                        pass.draw_rect(cx - 6.0, cy - 3.0, 8.0, 8.0, 1.0, icon_color);
                    } else {
                        pass.draw_rect(cx - 5.0, cy - 5.0, 10.0, 10.0, 1.0, icon_color);
                    }
                }
                TitleBarButton::Close => {
                    pass.draw_line(cx - 5.0, cy - 5.0, cx + 5.0, cy + 5.0, 1.0, icon_color);
                    pass.draw_line(cx - 5.0, cy + 5.0, cx + 5.0, cy - 5.0, 1.0, icon_color);
                }
            }
        }
    }
}
