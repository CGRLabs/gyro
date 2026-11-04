// Gyro's row in Options > Mods, with its flight controls.
fn on_mods(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use horizon_client::gyro as g;
    mods.push(crate::game::native_hooks::NativeMod {
        id: g::ID.into(),
        title: format!("Gyro {}", g::VERSION),
        description: g::DESCRIPTION.into(),
        keys: vec![
            ["roll".into(), "Roll (move the mouse left and right)".into(), "key.mouse.x".into()],
            ["pitch".into(), "Pitch (move the mouse up and down)".into(), "key.mouse.y".into()],
            ["rudder".into(), "Rudder (steer with A and D)".into(), "key.keyboard.a".into()],
        ],
    });
}
