use bevy::math::U16Vec2;
use bevy::prelude::*;
use bevy_terminal::TerminalTextOverlay;
use bevy_xpbd_2d::prelude::*;

use bevy_terminal::BackgroundColor;
use bevy_terminal::Camera;
use bevy_terminal::Color;
use bevy_terminal::Sprite;
use bevy_terminal::TerminalInput;
use bevy_terminal::TerminalPlugin;
use termios::Termios;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct GreenThing {
    pub velocity: Vec2,
    pub start_pos: Vec2,
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .insert_resource(BackgroundColor(Color {
            r: 59,
            g: 44,
            b: 65,
            a: 255,
        }))
        .insert_resource(TerminalTextOverlay {
            text: "Hello, world!".to_string(),
        })
        .insert_non_send_resource(LastBoxPos(Vec2::new(0.0, -20.0)))
        .insert_resource(Gravity(Vec2::NEG_Y * 150.))
        .add_plugins((TerminalPlugin, PhysicsPlugins::default()))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, move_sprite)
        .add_systems(FixedUpdate, move_green_thing)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        TransformBundle::from_transform(Transform::from_xyz(0.0, 0.0, 0.0)),
        Sprite::from_image(&image::open("examples/assets/player.png").unwrap(), 0),
        Player,
        RigidBody::Dynamic,
        Collider::rectangle(8.0, 8.0),
        LockedAxes::ROTATION_LOCKED,
    ));
    commands.spawn((
        Transform::from_translation(Vec3::new(10.0, 10.0, 0.0)),
        Sprite::from_image(&image::open("examples/assets/green.png").unwrap(), 1),
        GreenThing {
            velocity: Vec2::new(1.0, 0.0),
            start_pos: Vec2::new(10.0, 10.0),
        },
    ));
    commands.spawn((
        Transform::from_translation(Vec3::new(20.0, 20.0, 0.0)),
        Sprite::from_image(&image::open("examples/assets/red.png").unwrap(), 2),
    ));

    /* commands.spawn((
        TransformBundle::from_transform(Transform::from_xyz(0.0, -20.0, 0.0)),
        Sprite::from_image(&image::open("examples/assets/platform.png").unwrap(), 3),
        RigidBody::Static,
        Collider::rectangle(64.0, 8.0),
    )); */

    // camera
    commands.spawn((
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        Camera,
    ));
}

struct LastBoxPos(Vec2);

fn move_sprite(
    mut query: Query<(&mut LinearVelocity, &Transform), With<Player>>,
    input: Res<TerminalInput>,
    mut camera: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    mut text_overlay: ResMut<TerminalTextOverlay>,
    mut commands: Commands,
    mut last_box_pos: NonSendMut<LastBoxPos>,
) {
    for (mut velocity, transform) in query.iter_mut() {
        //transform.translation.x += 1.;

        let mut impulse = Vec2::ZERO;
        /*
        if input.active_keys.contains(&'a') {
            impulse.x -= 1.;
        }
        if input.active_keys.contains(&'d') {
            impulse.x += 1.;
        }*/
        impulse.x = 1.;

        if impulse.x != 0. {
            velocity.x = impulse.x * 50.;
        }

        if input.active_keys.contains(&'w') || input.active_keys.contains(&' ') {
            velocity.y = 60.;
        }

        // q to quit
        if input.active_keys.contains(&'q') {
            print!("\x1B[2J\x1B[1;1H\x1B[?25h"); // clear screen and show cursor
                                                 // disable raw mode
            let mut t = Termios::from_fd(0).unwrap();
            // reenable input echo
            t.c_lflag |= termios::ECHO | termios::ICANON;
            termios::tcsetattr(0, termios::TCSANOW, &t).unwrap();
            std::process::exit(0);
        }

        // smooth camera follow with lerping
        for mut camera_transform in camera.iter_mut() {
            camera_transform.translation = camera_transform.translation.lerp(
                Vec3::new(transform.translation.x, transform.translation.y, 0.0),
                0.05,
            );
            camera_transform.translation.x = transform.translation.x;
            // text overlay
            text_overlay.text = format!(
                "Player pos: ({:.2}, {:.2})",
                transform.translation.x, transform.translation.y
            );
        }

        // if player is 32 units away in X from last box, spawn a new box 64 units away
        if transform.translation.x - last_box_pos.0.x > -64. {
            let gap = 32.;
            commands.spawn((
                //Transform::from_translation(Vec3::new(last_box_pos.0.x + 64.0, -50.0, 0.0)),
                TransformBundle::from_transform(Transform::from_xyz(
                    last_box_pos.0.x + 64.0 + gap,
                    last_box_pos.0.y + 8.0,
                    0.0,
                )),
                Sprite::from_image(&image::open("examples/assets/platform.png").unwrap(), 3),
                RigidBody::Static,
                Collider::rectangle(64.0, 8.0),
            ));
            commands.spawn((
                //Transform::from_translation(Vec3::new(last_box_pos.0.x + 64.0, -50.0, 0.0)),
                TransformBundle::from_transform(Transform::from_xyz(
                    last_box_pos.0.x + 64.0 + gap,
                    last_box_pos.0.y + 16.0,
                    0.0,
                )),
                Sprite::from_image(&image::open("examples/assets/spike.png").unwrap(), 3),
                RigidBody::Static,
                Collider::triangle(Vec2::new(-3., -4.), Vec2::new(3., -4.), Vec2::new(0., 4.)),
            ));
            last_box_pos.0.x += 64.0 + gap;
            last_box_pos.0.y += 8.0;
        }
    }
}

fn move_green_thing(mut query: Query<(&mut GreenThing, &mut Transform)>) {
    // move in a square, as in like, first it moves 20px right, then 20px down, then 20px left, then 20px up
    for (mut green_thing, mut transform) in query.iter_mut() {
        // move by velocity
        let mut x = transform.translation.x;
        let mut y = transform.translation.y;
        x += green_thing.velocity.x;
        y += green_thing.velocity.y;

        // if our X is 20 greater than start, make velocity point down
        if x - green_thing.start_pos.x > 20. {
            green_thing.velocity = Vec2::new(0.0, -1.0);
        }
    }
}
