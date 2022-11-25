use crate::{structures::Align, ui, widget::HWidget};
use glib::GString;
use gtk::{traits::*, *};
use std::time::Duration;

/// Creates a new button widget.
pub struct ButtonWidget {
    pub tooltip: String,
    pub tooltip_command: String,
    pub command: String,
    pub button: Button,
    pub update_rate: u64,
    pub text_command: String,
}
fn start_text_loop(button: Button, text_command: String, update_rate: u64) {
    if text_command.is_empty() || update_rate == 0 {
        return;
    }

    let tick = move || {
        let mut new_text = String::default();
        new_text.push_str(execute!(&text_command).as_str());
        if !button.label().unwrap().eq(&new_text) {
            button.set_label(&new_text);
        }

        glib::Continue(true)
    };

    tick();
    glib::timeout_add_local(Duration::from_millis(update_rate), tick);
}

fn start_tooltip_loop(button: Button, tooltip: String, tooltip_command: String) {
    const EMPTY: &str = "";
    let tick = move || {
        let mut new_tooltip = String::default();
        new_tooltip.push_str(&tooltip);
        new_tooltip.push_str(execute!(&tooltip_command).as_str());

        let tooltip_markup = button
            .tooltip_markup()
            .unwrap_or_else(|| GString::from(EMPTY));

        if !tooltip_markup.eq(&new_tooltip) {
            // Markup support here, the user therefore has to deal with any upcoming issues due to
            // the command output, on their own.
            button.set_tooltip_markup(Some(&new_tooltip));
        }

        glib::Continue(true)
    };

    tick();
    // NOTE: This does NOT respect update_rate, since it's not meant to update super fast.
    glib::timeout_add_local(Duration::from_millis(1000), tick);
}

// Implements HWidget for the widget so that we can actually use it.
impl HWidget for ButtonWidget {
    fn add(self, name: String, align: Align, left: &Box, centered: &Box, right: &Box) {
        self.button.set_widget_name(&name);
        // 0.2.8: Support tooltips for buttons
        self.button.set_tooltip_markup(Some(&self.tooltip));
        // start loops
        self.start_loop();

        // If the command isn't empty, subscribe to click events.
        if !self.command.is_empty() {
            self.button.connect_clicked(move |_| {
                log!(format!("Button '{}' -> Clicked", name));
                execute!(&self.command);
            });
        }

        ui::add_and_align(&self.button, align, left, centered, right);
        log!("Added a new button widget");
    }

    fn start_loop(&self) {
        // 0.3.6: Support for commands on tooltips.
        if !self.tooltip_command.is_empty() {
            start_tooltip_loop(
                self.button.clone(),
                self.tooltip.clone(),
                self.tooltip_command.clone(),
            );
        }

        if !self.text_command.is_empty() {
            start_text_loop(
                self.button.clone(),
                self.text_command.clone(),
                self.update_rate.clone(),
            );
        }
    }
}
