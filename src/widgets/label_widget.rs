use crate::{config, structures::Align, ui, widget::HWidget};
use glib::GString;
use gtk::{traits::*, *};
use std::{process::Stdio, sync::RwLock, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    task,
};

lazy_static! {
    /// Current text buffer from `stdout`.
    static ref BUFFER: RwLock<String> = RwLock::new(String::default());
}

/// Creates a new label widget.
#[derive(Debug)]
pub struct LabelWidget {
    pub tooltip: String,
    pub tooltip_command: String,
    pub text: String,
    pub command: String,
    pub update_rate: u64,
    pub label: Label,
    pub listen: bool,
}

// For VEC to work.
unsafe impl Send for LabelWidget {}
unsafe impl Sync for LabelWidget {}

/// 0.3.2: If `listen` is `true`, call this function and then set the label text-value
///   to that of `BUFFER`.
fn begin_listen(cmd: String) {
    task::spawn(async move {
        let mut child = Command::new("bash")
            .args(["-c", &cmd])
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap_or_else(|_| panic!("[ERROR] Cannot start '{cmd}'\n"));

        let out = child
            .stdout
            .take()
            .expect("[ERROR] Cannot take stdout from child!\n");

        let mut reader = BufReader::new(out).lines();
        let update_rate = config::get_update_rate();
        loop {
            *BUFFER.write().unwrap() = reader
                .next_line()
                .await
                .expect("[ERROR] There are no more lines available!\n")
                .expect("[ERROR] The string value is None!\n");

            tokio::time::sleep(Duration::from_millis(update_rate)).await;
        }
    });
}

/// Starts updating the dynamic tooltip, if any.
fn start_tooltip_loop(label_ref: &LabelWidget) {
    let label = label_ref.label.clone();
    let tooltip = label_ref.tooltip.clone();
    let tooltip_command = label_ref.tooltip_command.clone();
    if tooltip_command.is_empty() {
        // Not eligible, cancel.
        return;
    }

    let tick = move || {
        let mut new_tooltip = String::default();
        new_tooltip.push_str(&tooltip);
        new_tooltip.push_str(execute!(&tooltip_command).as_str());

        let tooltip_markup = label.tooltip_markup().unwrap_or_else(|| GString::from(""));

        if !tooltip_markup.eq(&new_tooltip) {
            // Markup support here, the user therefore has to deal with any upcoming issues due to
            // the command output, on their own.
            label.set_tooltip_markup(Some(&new_tooltip));
        }

        glib::Continue(true)
    };

    tick();
    glib::timeout_add_local(Duration::from_millis(1000), tick);
}

/// Starts updating the dynamic label content.
fn start_label_loop(label: Label, text: String, command: String, update_rate: u64, listen: bool) {
    if command.is_empty() || update_rate == 0 {
        // Not eligible, cancel.
        return;
    }

    let tick = move || {
        if !listen {
            let mut new_text = String::default();
            new_text.push_str(&text);
            new_text.push_str(execute!(&command).as_str());

            if !label.text().eq(&new_text) {
                // Not the same as new_text; redraw.
                label.set_text(&new_text);
            }
        } else {
            update_from_buffer(&label);
        }

        glib::Continue(true)
    };

    tick();
    glib::timeout_add_local(Duration::from_millis(update_rate), tick);
}

/// Updates the labels content with the string from `BUFFER`.
fn update_from_buffer(label: &Label) {
    let new_content = BUFFER
        .read()
        .expect("[ERROR] Failed retrieving content from BUFFER!\n");
    let old_content = label.text();
    // eq-check the new content for old_content. Doing the opposite requires a .to_string()
    // call.
    if !new_content.eq(&old_content) {
        // Not the same; set content and redraw.
        label.set_text(&new_content);
    }
}

// Implements HWidget for the widget so that we can actually use it.
impl HWidget for LabelWidget {
    fn add(self, name: String, align: Align, left: &Box, centered: &Box, right: &Box) {
        let is_static = self.command.is_empty() || self.update_rate == 0;
        self.label.set_widget_name(&name);
        self.label.set_tooltip_markup(Some(&self.tooltip));
        ui::add_and_align(&self.label, align, left, centered, right);

        if self.listen {
            begin_listen(self.command.clone());
        }

        self.start_loop();

        if is_static {
            self.label.set_markup(&self.text);
        }

        log!(format!(
            "Added a new label widget named '{name}', is static: {}",
            is_static
        ));
    }

    fn start_loop(&self) {
        // Start loops.
        start_tooltip_loop(self);

        start_label_loop(
            self.label.clone(),
            self.text.clone(),
            self.command.clone(),
            self.update_rate,
            self.listen,
        );
    }
}
