// welcome to bevy_terminal, we render bevy game to terminal

use bevy::{math::U16Vec2, prelude::*};
use image::DynamicImage;

use std::io;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::{thread, time};
use tty_read::{ReaderOptions, TermReader};

#[derive(Default, Copy, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

// implement partialeq for color
impl PartialEq for Color {
    fn eq(&self, other: &Self) -> bool {
        self.r == other.r && self.g == other.g && self.b == other.b && self.a == other.a
    }
}

// so the way this works is, we have a Sprite component that we can attach to entities, this component has just a bunch of colors, size, and z index
// then we will go through and render them one at a time starting with the lower ones to a buffer (this allows alpha blending)
// lastly we will print the buffer to the terminal

#[derive(Component)]
pub struct Sprite {
    pub colors: Vec<Color>,
    /// If the `x` of your `size` is inaccurate, the sprite will be warped. However, if the `y` is inaccurate, the sprite will simply be cut off or padded with empty space.
    pub size: U16Vec2,
    pub z_index: i32,
}

impl Sprite {
    // they supply a DynamicImage, we can use this to create a sprite
    pub fn from_image(image: &DynamicImage, z_index: i32) -> Self {
        let image = image.to_rgba8();
        let (width, height) = image.dimensions();
        let mut colors = Vec::new();
        for y in 0..height {
            for x in 0..width {
                let pixel = image.get_pixel(x, y);
                colors.push(Color {
                    r: pixel[0],
                    g: pixel[1],
                    b: pixel[2],
                    a: pixel[3],
                });
            }
        }
        Self {
            colors,
            size: U16Vec2::new(width as u16, height as u16),
            z_index,
        }
    }
}

#[derive(Resource)]
pub struct BackgroundColor(pub Color);

#[derive(Default)]
pub struct TerminalPlugin;

// resource for our stdin
pub struct Input {
    pub receiver: Receiver<char>,
}

#[derive(Resource)]
pub struct TerminalInput {
    pub active_keys: Vec<char>,
}

#[derive(Resource)]
pub struct TerminalTextOverlay {
    pub text: String,
}

#[derive(Component)]
pub struct Camera;

impl Plugin for TerminalPlugin {
    fn build(&self, app: &mut App) {
        //app.add_systems(Startup, setup_input);
        /*app.insert_non_send_resource(Input {
            receiver: mpsc::channel().1,
        });*/

        app.add_systems(FixedUpdate, render);

        // Jesse we need to setup the input system
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            // Configure reader options
            let options = ReaderOptions::default();

            // Open a reader
            let reader = TermReader::open_stdin(&options).expect("failed to open stdin reader");

            loop {
                let mut buffer = [0; 1];
                /*match io::Read::read(&mut io::stdin(), &mut buffer) {
                    Ok(_) => {
                        sender.send(buffer[0] as char).unwrap();
                    }
                    Err(error) => {
                        eprintln!("error: {}", error);
                    }
                }*/
                match reader.read_byte() {
                    Ok(byte) => {
                        sender.send(byte as char).unwrap();
                    }
                    Err(error) => {
                        eprintln!("error: {}", error);
                    }
                }
            }
        });
        app.insert_non_send_resource(Input { receiver });
        app.insert_resource(TerminalInput {
            active_keys: Vec::new(),
        });

        app.add_systems(FixedPreUpdate, input);
    }
}

fn input(mut input: NonSendMut<Input>, mut terminal_input: ResMut<TerminalInput>) {
    terminal_input.active_keys.clear();
    loop {
        match input.receiver.try_recv() {
            Ok(key) => {
                terminal_input.active_keys.push(key);
            }
            Err(TryRecvError::Empty) => {
                break;
            }
            Err(TryRecvError::Disconnected) => {
                break;
            }
        }
    }
    //println!("{:?}", terminal_input.active_keys);
}

fn render(
    mut sprites: Query<(&Sprite, &Transform)>,
    background_color: Res<BackgroundColor>,
    camera: Query<&Transform, With<Camera>>,
    text_overlay: Res<TerminalTextOverlay>,
) {
    if let Some((w, h)) = term_size::dimensions() {
        let h = h * 2;
        let mut string_buffer = "\x1b[1;1H".to_string(); // clear screen
                                                         // hide cursor
        string_buffer.push_str("\x1b[?25l");
        /*let mut buffer = vec![vec![Color::default(); w as usize]; h as usize];
        for (sprite, transform) in sprites.iter() {
            let x = transform.translation.x as i32;
            let y = transform.translation.y as i32;
            let x = x.max(0).min(w as i32);
            let y = y.max(0).min(h as i32);
            for (i, color) in sprite.colors.iter().enumerate() {
                let x = x + i as i32 % sprite.size.x as i32;
                let y = y + i as i32 / sprite.size.x as i32;
                if x < 0 || x >= w as i32 || y < 0 || y >= h as i32 {
                    continue;
                }
                buffer[y as usize][x as usize] = *color;
            }
        }
        for i in 0..h {
            let row = &buffer[i as usize];
            for color in row {
                string_buffer.push_str(&format!("\x1b[48;2;{};{};{}m ", color.r, color.g, color.b));
            }
            string_buffer.push_str("\x1b[0m");
            if i != h - 1 {
                string_buffer.push_str("\n");
            }
        }
        print!("{}", string_buffer);*/

        // we are switching to halfblock rendering, this means we can render 2 pixels in one character with '▄'. we set the foreground with [38;2;{r};{g};{b}m and the background with [48;2;{r};{g};{b}m. fg is bottom, bg is top
        // everything the same way except we loop over every 2 rows instead of every row, and we use them together to form a halfblock
        let mut buffer = vec![vec![background_color.0; w as usize]; h as usize];
        let camera = camera.iter().next().unwrap().translation;
        /*for (sprite, transform) in sprites.iter() {
            let x = transform.translation.x as i32;
            let y = -transform.translation.y as i32;

            let x = x.max(0).min(w as i32);
            let y = y.max(0).min(h as i32);
            for (i, color) in sprite.colors.iter().enumerate() {
                let x = x + i as i32 % sprite.size.x as i32;
                let y = y + i as i32 / sprite.size.x as i32;
                if x < 0 || x >= w as i32 || y < 0 || y >= h as i32 {
                    continue;
                }
                //buffer[y as usize][x as usize] = *color;
                // Its not THat easy we need to blend the colors
                let bg = &mut buffer[y as usize][x as usize];
                let fg = color;
                let alpha = fg.a as f32 / 255.0;
                bg.r = (bg.r as f32 * (1.0 - alpha) + fg.r as f32 * alpha) as u8;
                bg.g = (bg.g as f32 * (1.0 - alpha) + fg.g as f32 * alpha) as u8;
                bg.b = (bg.b as f32 * (1.0 - alpha) + fg.b as f32 * alpha) as u8;
            }
        }*/
        // just like above but Two changes:
        // 1. use camera to offset the sprites, a sprite at 0,0 will be in the center of the screen if the camera is at 0,0 too
        // 2. we no longer clamp positions, we will just skip offscreen pixels
        for (sprite, transform) in sprites.iter() {
            //let x = (transform.translation.x - camera.x) as i32 + w as i32 / 2;
            //let y = -(transform.translation.y - camera.y) as i32 + h as i32 / 2;
            // almost forgot, we want sprite origin to be at the center
            /*let x = (transform.translation.x - camera.x) as i32 + w as i32 / 2
                - sprite.size.x as i32 / 2;
            let y = -(transform.translation.y - camera.y) as i32 + h as i32 / 2
                - sprite.size.y as i32 / 2;*/

            // One More thing, when values are decimal, we dont want inconsistent positioning, so we will round them
            let camera_x = camera.x.round() as i32;
            let camera_y = camera.y.round() as i32;
            let x = (transform.translation.x.round() as i32 - camera_x) + w as i32 / 2
                - sprite.size.x as i32 / 2;
            let y = -(transform.translation.y.round() as i32 - camera_y) + h as i32 / 2
                - sprite.size.y as i32 / 2;

            for (i, color) in sprite.colors.iter().enumerate() {
                let x = x + i as i32 % sprite.size.x as i32;
                let y = y + i as i32 / sprite.size.x as i32;
                if x < 0 || x >= w as i32 || y < 0 || y >= h as i32 {
                    continue;
                }
                let bg = &mut buffer[y as usize][x as usize];
                let fg = color;
                let alpha = fg.a as f32 / 255.0;
                bg.r = (bg.r as f32 * (1.0 - alpha) + fg.r as f32 * alpha) as u8;
                bg.g = (bg.g as f32 * (1.0 - alpha) + fg.g as f32 * alpha) as u8;
                bg.b = (bg.b as f32 * (1.0 - alpha) + fg.b as f32 * alpha) as u8;
            }
        }

        let mut prev_fg = Color::default();
        let mut prev_bg = Color::default();

        for i in (0..h).step_by(2) {
            let row = &buffer[i as usize];
            let row2 = &buffer[i as usize + 1];
            for (bg, fg) in row.iter().zip(row2.iter()) {
                /*string_buffer.push_str(&format!(
                    "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m▄",
                    color.r, color.g, color.b, color2.r, color2.g, color2.b
                ));*/

                let mut pixel = "".to_string();
                if *fg != prev_fg {
                    pixel.push_str(&format!("\x1b[38;2;{};{};{}m", fg.r, fg.g, fg.b));
                    prev_fg = *fg;
                }
                if *bg != prev_bg {
                    pixel.push_str(&format!("\x1b[48;2;{};{};{}m", bg.r, bg.g, bg.b));
                    prev_bg = *bg;
                }
                pixel.push_str("▄");
                string_buffer.push_str(&pixel);
                /*
                let new_pixel = format!(
                    "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m▄",
                    color.r, color.g, color.b, color2.r, color2.g, color2.b
                );
                if new_pixel != prev_pixel {
                    string_buffer.push_str(&new_pixel);
                    prev_pixel = new_pixel;
                } else {
                    string_buffer.push_str("▄");
                }*/
            }
        }

        // lastly, move back to top left and show text overlay
        string_buffer.push_str("\x1b[0m\x1b[1;1H");
        string_buffer.push_str(&text_overlay.text);

        //print!("{}", string_buffer);
        // we use stderr so if user checks stdout they will only get logs instead of loads of rendering
        eprint!("{}", string_buffer);
    } else {
        return;
    }
}
