// Register Gyro in Options > Mods
fn on_mods(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: "gyro".into(),
        title: "Gyro 1.4.0".into(),
        description: "True flight simulator 6-DOF elytra aerobatics: avatar locked to the camera roll, Mouse X rolls, Mouse Y pitches, A/D yaws.".into(),
        keys: vec![
            ["roll".into(), "Roll (Mouse X)".into(), "key.mouse.x".into()],
            ["pitch".into(), "Pitch (Mouse Y)".into(), "key.mouse.y".into()],
            ["rudder".into(), "Rudder Yaw (A/D)".into(), "key.keyboard.a".into()],
        ],
    });
}
