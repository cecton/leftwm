use super::{CommandBuilder, Config, DisplayEvent, Manager, Mode};
use crate::utils;
use crate::{display_action::DisplayAction, models::FocusBehaviour};

impl<C: Config<CMD>, CMD> Manager<C, CMD> {
    /// Process a collection of events, and apply them changes to a manager.
    /// Returns true if changes need to be rendered.
    pub fn display_event_handler(&mut self, event: DisplayEvent<CMD>) -> bool {
        let update_needed = match event {
            DisplayEvent::ScreenCreate(s) => self.screen_create_handler(s),
            DisplayEvent::WindowCreate(w, x, y) => self.window_created_handler(w, x, y),
            DisplayEvent::WindowChange(w) => self.window_changed_handler(w),

            //The window has been focused, do we want to do anything about it?
            DisplayEvent::MouseEnteredWindow(handle) => match self.focus_manager.behaviour {
                FocusBehaviour::Sloppy => return self.focus_window(&handle),
                _ => return false,
            },

            DisplayEvent::MoveFocusTo(x, y) => self.move_focus_to_point(x, y),

            //This is a request to validate focus. Double check that we are focused the correct
            //thing under this point.
            DisplayEvent::VerifyFocusedAt(x, y) => match self.focus_manager.behaviour {
                FocusBehaviour::Sloppy => return self.validate_focus_at(x, y),
                _ => return false,
            },

            DisplayEvent::WindowDestroy(handle) => self.window_destroyed_handler(&handle),

            DisplayEvent::KeyCombo(mod_mask, xkeysym) => {
                //look through the config and build a command if its defined in the config
                let build = CommandBuilder::<C, CMD>::new(&self.config);
                let command = build.xkeyevent(mod_mask, xkeysym);
                if let Some((cmd, val)) = command {
                    self.command_handler(cmd, val)
                } else {
                    false
                }
            }

            DisplayEvent::SendCommand(command, value) => {
                self.command_handler(&command, value.as_deref())
            }

            DisplayEvent::MouseCombo(mod_mask, button, handle) => {
                let mouse_key = utils::xkeysym_lookup::into_mod(self.config.mousekey());
                self.mouse_combo_handler(mod_mask, button, handle, mouse_key)
            }

            DisplayEvent::ChangeToNormalMode => {
                self.mode = Mode::Normal;
                //look through the config and build a command if its defined in the config
                let act = DisplayAction::NormalMode;
                self.actions.push_back(act);
                true
            }

            DisplayEvent::Movement(handle, x, y) => {
                if self.screens.iter().any(|s| s.root == handle)
                    && self.focus_manager.behaviour == FocusBehaviour::Sloppy
                {
                    return self.focus_workspace_under_cursor(x, y);
                }
                false
            }

            DisplayEvent::MoveWindow(handle, time, x, y) => {
                //limit the frame rate to 60f/sec. otherwise you get lag
                let mut refresh = false;
                if (time - self.frame_rate_limitor) > (1000 / 60) {
                    refresh = self.window_move_handler(&handle, x, y);
                    self.frame_rate_limitor = time;
                }
                refresh
            }
            DisplayEvent::ResizeWindow(handle, time, x, y) => {
                //limit the frame rate to 60f/sec. otherwise you get lag
                let mut refresh = false;
                if (time - self.frame_rate_limitor) > (1000 / 60) {
                    refresh = self.window_resize_handler(&handle, x, y);
                    self.frame_rate_limitor = time;
                }
                refresh
            }
        };

        if update_needed {
            self.update_windows();
        }

        update_needed
    }
}
