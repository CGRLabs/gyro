// OVERWRITE hook for Game::apply_mouse_look:
// True "Do A Gyro" flight controls:
// - While flying with Elytra:
//   Mouse X directly controls roll (barrel roll & bank freely).
//   Mouse Y controls elevator pitch.
// - While on ground / walking:
//   Standard vanilla mouse look (yaw & pitch).

fn on_mouse_look(game: &mut crate::game::Game<'_>) {
    let look = game.look_options();
    let time = game.started.elapsed().as_secs_f64();
    let pointer = game.captured || game.agent_pointer;
    let is_playing = match game.screen {
        crate::game::Screen::Playing => true,
        _ => false,
    };
    let grabbed = pointer && is_playing && game.player.is_some();
    let active = game.focused || game.agent_pointer;
    if let Some((xo, yo)) = game
        .mouse
        .handle_accumulated_movement(time, active, grabbed, look)
    {
        game.tutorial.mouse(xo, yo);
        let zoom = game.zoom_turn_scale();
        let xo = xo * zoom;
        let yo = yo * zoom;

        let gliding = match &game.player {
            Some(p) => p.fall_flying,
            None => false,
        };

        if gliding {
            let cur_roll = horizon_client::gyro::get_roll();
            let (pitch_delta, yaw_delta, new_roll) =
                horizon_client::gyro::calculate_flight_turn(xo, yo, cur_roll);
            horizon_client::gyro::set_roll(new_roll);

            let mut new_pitch = game.pitch + pitch_delta;
            let mut new_yaw = game.yaw + yaw_delta;

            // 360-degree vertical loop-de-loop:
            // When pitch passes straight up or straight down, flip yaw and roll smoothly:
            if new_pitch > 90.0 {
                new_pitch = 180.0 - new_pitch;
                new_yaw = (new_yaw + 180.0).rem_euclid(360.0);
                let r = horizon_client::gyro::get_roll();
                horizon_client::gyro::set_roll((r + 180.0).rem_euclid(360.0));
            } else if new_pitch < -90.0 {
                new_pitch = -180.0 - new_pitch;
                new_yaw = (new_yaw + 180.0).rem_euclid(360.0);
                let r = horizon_client::gyro::get_roll();
                horizon_client::gyro::set_roll((r + 180.0).rem_euclid(360.0));
            }

            new_pitch = new_pitch.clamp(-90.0, 90.0);
            let p_diff = new_pitch - game.pitch;
            game.pitch = new_pitch;
            game.yaw = new_yaw;

            if let Some(m) = game.player_model.as_mut() {
                m.previous_head_pitch = (m.previous_head_pitch + p_diff).clamp(-90.0, 90.0);
                m.head_pitch = new_pitch;
            }
        } else {
            // Normal walking / sitting / riding controls
            game.turn_player(xo, yo);
        }
    }
}
