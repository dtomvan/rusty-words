use fltk::{
    app::{App, Scheme},
    button::Button,
    prelude::*,
    window::Window,
};
use fltk_theme::{color_themes, ColorTheme};

fn main() {
    let a = App::default().with_scheme(Scheme::Base);
    let scheme = fltk_theme::WidgetScheme::new(fltk_theme::SchemeType::Aqua);
    scheme.apply();
    let theme = ColorTheme::new(color_themes::BLACK_THEME);
    theme.apply();
    let mut win = Window::default().with_size(400, 300);
    let mut btn = Button::new(160, 200, 80, 40, "Hello");
    btn.set_color(btn.color().lighter());
    win.end();
    win.show();
    a.run().unwrap();
}
