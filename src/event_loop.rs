use crate::{config::Config, models::FocusBehaviour};
use crate::{CommandPipe, DisplayServer, Manager, Mode, Window, Workspace};
use std::sync::atomic::Ordering;

impl<C: Config, SERVER: DisplayServer> Manager<C, SERVER> {
    pub async fn event_loop(mut self) {
        let mut command_pipe = CommandPipe::new(self.state.config.pipe_file())
            .await
            .expect("ERROR: couldn't connect to commands.pipe");

        //main event loop
        let mut event_buffer = vec![];
        let mut startup = true;
        loop {
            if self.state.mode == Mode::Normal {
                self.state.config.on_state_update((&self).into()).await;
            }
            self.display_server.flush();

            let mut needs_update = false;
            tokio::select! {
                _ = self.display_server.wait_readable(), if event_buffer.is_empty() => {
                    event_buffer.append(&mut self.display_server.get_next_events());
                    continue;
                }
                //Once in a blue moon we miss the focus event,
                //This is to double check that we know which window is currently focused
                _ = timeout(100), if event_buffer.is_empty() && self.state.focus_manager.behaviour == FocusBehaviour::Sloppy => {
                    let mut focus_event = self.display_server.verify_focused_window();
                    event_buffer.append(&mut focus_event);
                    continue;
                }
                Some(cmd) = command_pipe.read_command(), if event_buffer.is_empty() => {
                    needs_update = self.command_handler(&cmd) || needs_update;
                    self.display_server.update_theme_settings(&self.state.config);
                }
                else => {
                    event_buffer.drain(..).for_each(|event| needs_update = self.display_event_handler(event) || needs_update);
                }
            }

            //if we need to update the displayed state
            if needs_update {
                match self.state.mode {
                    Mode::Normal => {
                        let windows: Vec<&Window> = self.state.windows.iter().collect();
                        let focused = self.focused_window();
                        self.display_server.update_windows(windows, focused, &self);
                        let workspaces: Vec<&Workspace> = self.state.workspaces.iter().collect();
                        let focused = self.focused_workspace();
                        self.display_server.update_workspaces(workspaces, focused);
                    }
                    //when (resizing / moving) only deal with the single window
                    Mode::ResizingWindow(h) | Mode::MovingWindow(h) => {
                        let focused = self.focused_window();
                        let windows: Vec<&Window> = (&self.state.windows)
                            .iter()
                            .filter(|w| w.handle == h)
                            .collect();
                        self.display_server.update_windows(windows, focused, &self);
                    }
                }
            }

            //preform any actions requested by the handler
            while !self.state.actions.is_empty() {
                if let Some(act) = self.state.actions.pop_front() {
                    if let Some(event) = self.display_server.execute_action(act) {
                        event_buffer.push(event);
                    }
                }
            }

            //after the very first loop run the 'up' scripts (global and theme). we need the unix
            //socket to already exist.
            if startup {
                startup = false;
                C::on_startup(&mut self).await;
                C::load_state(&mut self);
            }

            if self.reap_requested.swap(false, Ordering::SeqCst) {
                self.children.reap();
            }

            if self.reload_requested {
                break;
            }
        }

        self.state.config.on_shutdown().await;
    }
}

async fn timeout(mills: u64) {
    use tokio::time::{sleep, Duration};
    sleep(Duration::from_millis(mills)).await;
}
