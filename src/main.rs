use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::time::Real;
use bevy::ui::IsDefaultUiCamera;

const FLOOR: f32 = -0.30;
const FISH_MAX: usize = 13;
const NX: usize = 72; // segments along the body
const NY: usize = 28; // segments top to bottom

#[derive(Resource, Clone)]
struct Config {
    fish: usize,    // individual fish (0..=FISH_MAX)
    school: usize,  // green chromis in the school (0..=7)
    grass: usize,   // seagrass clumps
    bubbles: usize, // bubbles in the streams
    motes: usize,   // drifting plankton specks
    day_min: f32,   // minutes for a full day/night cycle
    daynight: bool, // day/night cycle on (off by default)
    speed: f32,     // overall swim/sway speed multiplier
    no_effects: bool,
    windowed: bool,
    render_w: u32,
    shark: bool,    // an occasional shark visit
}

const HELP: &str = "Reef aquarium. Settings you choose are remembered for next time.

Usage: aquarium [options]
  --windowed / --fullscreen   run in a window / fill the screen (default)
  --fish N              individual fish, 0-13 (default 10)
  --school N            small green fish in the school, 0-7 (default 7)
  --grass N             seagrass clumps, 0-40 (default 16; 0 = none)
  --bubbles N           bubbles in the rising streams, 0-120 (default 45; 0 = none)
  --motes N             drifting plankton specks, 0-300 (default 90; 0 = none)
  --day-night           turn on the slow day and night cycle (off by default; --no-day-night turns it off)
  --day-length MIN      minutes for a full day and night (default 30); also turns the cycle on
  --speed X             speed of swimming and sway, 0.2-3 (default 1.0)
  --render-width N      internal render width in pixels (default 1600; lower = faster)
  --no-shark            no shark visits (--shark turns them back on; they are rare: one every 10-25 minutes)
  --no-effects          turn off light rays and caustic shimmer
  --reset               ignore and overwrite the remembered settings
  --no-vsync            do not wait for the display refresh (benchmarking)
  --fps                 print the average frame rate every 10 s
  --help                show this text

Keys while running:
  F / D        add / remove a fish        Up / Right   faster
  P            pause / resume             Down / Left  slower
  E            feed the fish              K  call the shark now   Shift+K  sharks on / off
  S            save a screenshot to screenshots/
  N            day/night cycle on / off     Shift+N  jump ahead in the day
  F11          fullscreen / window        Esc or Q     quit

Settings file: ~/.config/reef-aquarium/settings.conf (Windows: %APPDATA%\\reef-aquarium)
";

fn parse_args() -> Config {
    let mut c = Config { fish: 10, school: 7, grass: 16, bubbles: 45, motes: 90, day_min: 30.0, daynight: false, speed: 1.0, no_effects: false, windowed: false, render_w: 1600, shark: true };
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.iter().any(|a| a == "--reset") { load_settings(&mut c); }
    let mut i = 0;
    let num = |i: usize, name: &str| -> f32 {
        match args.get(i + 1).and_then(|v| v.parse::<f32>().ok()) {
            Some(v) => v,
            None => { eprintln!("{name} needs a number (try --help)"); std::process::exit(2); }
        }
    };
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => { print!("{HELP}"); std::process::exit(0); }
            "--windowed" => c.windowed = true,
            "--fullscreen" => c.windowed = false,
            "--reset" => {}
            "--no-vsync" => unsafe { std::env::set_var("AQUARIUM_NOVSYNC", "1") },
            "--fps" => unsafe { std::env::set_var("AQUARIUM_FPS", "1") },
            "--no-effects" => c.no_effects = true,
            "--shark" => c.shark = true,
            "--no-shark" => c.shark = false,
            "--fish" => { c.fish = num(i, "--fish").clamp(0.0, FISH_MAX as f32) as usize; i += 1; }
            "--school" => { c.school = num(i, "--school").clamp(0.0, 7.0) as usize; i += 1; }
            "--grass" => { c.grass = num(i, "--grass").clamp(0.0, 40.0) as usize; i += 1; }
            "--bubbles" => { c.bubbles = num(i, "--bubbles").clamp(0.0, 120.0) as usize; i += 1; }
            "--motes" => { c.motes = num(i, "--motes").clamp(0.0, 300.0) as usize; i += 1; }
            "--day-length" => { c.day_min = num(i, "--day-length").clamp(1.0, 1440.0); c.daynight = true; i += 1; }
            "--day-night" => c.daynight = true,
            "--no-day-night" => c.daynight = false,
            "--speed" => { c.speed = num(i, "--speed").clamp(0.2, 3.0); i += 1; }
            "--render-width" => { c.render_w = num(i, "--render-width").clamp(480.0, 3840.0) as u32; i += 1; }
            other => { eprintln!("unknown option {other} (try --help)"); std::process::exit(2); }
        }
        i += 1;
    }
    unsafe {
        if c.windowed { std::env::set_var("AQUARIUM_WINDOWED", "1"); }
        std::env::set_var("AQUARIUM_RENDER_W", c.render_w.to_string());
    }
    save_settings(&c);
    c
}

fn saving_enabled() -> bool {
    std::env::var("SHOT").is_err() && std::env::var("AQUARIUM_NOSAVE").is_err()
}

fn settings_path() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(std::path::PathBuf::from)) // Windows
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))?;
    Some(base.join("reef-aquarium").join("settings.conf"))
}

/// Remembered settings: one `key=value` per line. Unknown or bad lines are ignored.
fn load_settings(c: &mut Config) {
    let Some(path) = settings_path() else { return };
    let Ok(text) = std::fs::read_to_string(path) else { return };
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "fish" => if let Ok(n) = v.parse::<usize>() { c.fish = n.min(FISH_MAX) },
            "school" => if let Ok(n) = v.parse::<usize>() { c.school = n.min(7) },
            "grass" => if let Ok(n) = v.parse::<usize>() { c.grass = n.min(40) },
            "bubbles" => if let Ok(n) = v.parse::<usize>() { c.bubbles = n.min(120) },
            "motes" => if let Ok(n) = v.parse::<usize>() { c.motes = n.min(300) },
            "day_length" => if let Ok(x) = v.parse::<f32>() { c.day_min = x.clamp(1.0, 1440.0) },
            "day_night" => c.daynight = v == "on",
            "speed" => if let Ok(x) = v.parse::<f32>() { c.speed = x.clamp(0.2, 3.0) },
            "effects" => c.no_effects = v == "off",
            "windowed" => c.windowed = v == "true",
            "shark" => c.shark = v == "on",
            "render_width" => if let Ok(n) = v.parse::<u32>() { c.render_w = n.clamp(480, 3840) },
            _ => {}
        }
    }
}

fn save_settings(c: &Config) {
    if !saving_enabled() { return; }
    let Some(path) = settings_path() else { return };
    if let Some(dir) = path.parent() { let _ = std::fs::create_dir_all(dir); }
    let text = format!(
        "fish={}\nschool={}\ngrass={}\nbubbles={}\nmotes={}\nday_length={}\nday_night={}\nspeed={:.2}\neffects={}\nwindowed={}\nrender_width={}\nshark={}\n",
        c.fish, c.school, c.grass, c.bubbles, c.motes, c.day_min, if c.daynight { "on" } else { "off" }, c.speed, if c.no_effects { "off" } else { "on" }, c.windowed, c.render_w, if c.shark { "on" } else { "off" });
    let _ = std::fs::write(path, text);
}

fn main() {
    let cfg = parse_args();
    // Fullscreen by default; AQUARIUM_WINDOWED=1 (or a SHOT capture) runs in a window.
    let windowed = std::env::var("AQUARIUM_WINDOWED").is_ok() || std::env::var("SHOT").is_ok();
    let mode = if windowed {
        bevy::window::WindowMode::Windowed
    } else {
        bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Primary)
    };
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "Aquarium".into(), mode, present_mode: if std::env::var("AQUARIUM_NOVSYNC").is_ok() { bevy::window::PresentMode::AutoNoVsync } else { bevy::window::PresentMode::AutoVsync }, ..default() }),
            primary_cursor_options: Some(bevy::window::CursorOptions { visible: windowed, ..default() }),
            ..default()
        }))
        .insert_resource(DayCycle { offset: 0.33, d: 1.0, blend: 0.0 })
        .insert_resource(cfg)
        .insert_resource(ClearColor(Color::srgb(0.09, 0.36, 0.50)))
        .add_systems(Startup, setup)
        .add_systems(PostStartup, hide_fx)
        .add_systems(Startup, setup_osd)
        .insert_resource(Osd::default())
        .insert_resource(FeedReq(false))
        .insert_resource(SharkClock { next: 0.0, summon: false })
        .add_systems(Update, (debug_triggers, feed_start, animate_flakes, shark_ai))
        .add_systems(Update, (swim, drift, school_target, sway_grass, animate_bubbles, light_rocks, update_osd, crab_walk, animate_motes, camera_drift, day_night, auto_shot, animate_light, controls, fps_log, auto_quality, fit_upscale))
        .run();
}

/// Keys: Esc/Q quit, F11 fullscreen, F/D more/fewer fish, P pause, S screenshot, arrows change speed.
fn controls(
    keys: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
    mut windows: Query<(&mut Window, &mut bevy::window::CursorOptions), With<bevy::window::PrimaryWindow>>,
    mut cfg: ResMut<Config>,
    mut vtime: ResMut<Time<Virtual>>,
    mut commands: Commands,
    mut osd: ResMut<Osd>,
    real: Res<Time<Real>>,
    mut day: ResMut<DayCycle>,
    mut feed: ResMut<FeedReq>,
    mut clock: ResMut<SharkClock>,
) {
    let now = real.elapsed_secs();
    if keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyQ) {
        save_settings(&cfg);
        exit.write(AppExit::Success);
    }
    let mut changed = false;
    if keys.just_pressed(KeyCode::F11) {
        if let Ok((mut w, mut cur)) = windows.single_mut() {
            let to_window = !matches!(w.mode, bevy::window::WindowMode::Windowed);
            w.mode = if to_window { bevy::window::WindowMode::Windowed } else {
                bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Primary)
            };
            cur.visible = to_window;
            cfg.windowed = to_window;
            changed = true;
        }
    }
    if keys.just_pressed(KeyCode::KeyF) && cfg.fish < FISH_MAX {
        cfg.fish += 1;
        println!("fish: {}", cfg.fish);
        osd.show_hud(now, "");
        changed = true;
    }
    if keys.just_pressed(KeyCode::KeyD) && cfg.fish > 0 {
        cfg.fish -= 1;
        println!("fish: {}", cfg.fish);
        osd.show_hud(now, "");
        changed = true;
    }
    if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::ArrowRight) {
        cfg.speed = (cfg.speed * 1.25).min(3.0);
        println!("speed: {:.2}", cfg.speed);
        osd.show_hud(now, "");
        changed = true;
    }
    if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::ArrowLeft) {
        cfg.speed = (cfg.speed / 1.25).max(0.2);
        println!("speed: {:.2}", cfg.speed);
        osd.show_hud(now, "");
        changed = true;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        if vtime.is_paused() { vtime.unpause(); println!("resumed"); } else { vtime.pause(); println!("paused"); }
        osd.show_hud(now, "");
    }
    if keys.just_pressed(KeyCode::KeyS) {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = std::fs::create_dir_all("screenshots");
        let path = format!("screenshots/aquarium-{secs}.png");
        println!("screenshot: {path}");
        osd.show_hud(now, "Screenshot saved");
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    }
    if keys.just_pressed(KeyCode::KeyE) {
        feed.0 = true;
        osd.show_hud(now, "Feeding time");
    }
    if keys.just_pressed(KeyCode::KeyK) {
        if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
            cfg.shark = !cfg.shark;
            osd.show_hud(now, if cfg.shark { "Shark visits: on" } else { "Shark visits: off" });
            changed = true;
        } else {
            clock.summon = true;
            osd.show_hud(now, "A shadow in the distance...");
        }
    }
    if keys.just_pressed(KeyCode::KeyN) {
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        if shift && cfg.daynight {
            day.offset = (day.offset + 0.125).fract();
            let ph = day_phase(&day, vtime.elapsed_secs() * cfg.speed, cfg.day_min);
            let hour = (ph * 24.0).floor() as u32 % 24;
            osd.show_hud(now, &format!("Time of day  {:02}:00", hour));
        } else {
            cfg.daynight = !cfg.daynight;
            osd.show_hud(now, if cfg.daynight { "Day / night cycle: on" } else { "Day / night cycle: off" });
            println!("day/night: {}", if cfg.daynight { "on" } else { "off" });
            changed = true;
        }
    }
    if keys.just_pressed(KeyCode::Slash) || keys.just_pressed(KeyCode::KeyH) || keys.just_pressed(KeyCode::F1) {
        osd.help_until = if osd.help_until > now { 0.0 } else { now + 8.0 };
    }
    if changed { save_settings(&cfg); }
}

#[derive(Component)]
struct Fx;

#[derive(Component)]
struct Upscale;

#[derive(Component)]
struct Ray { x: f32, phase: f32, k: f32 }

#[derive(Resource)]
struct CausticMat(Handle<StandardMaterial>);

fn repeat_img(assets: &AssetServer, path: &'static str) -> Handle<Image> {
    assets.load_builder().with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
        s.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
            address_mode_u: bevy::image::ImageAddressMode::Repeat,
            address_mode_v: bevy::image::ImageAddressMode::Repeat,
            ..default()
        });
    }).load(path)
}

#[derive(Component)]
struct Fish {
    len: f32,
    height: f32,
    depth: f32,
    phase: f32,
    home: Vec3,
    speed: f32,
    target: Vec3,
    wait: f32,
    yaw: f32,
    vel: f32,
    rng: u32,
    dart: f32,
    school: bool,
    offset: Vec3,
    thick: &'static [u8],
    dir: f32,
    hover: f32,
    vy: f32,
    pitch: f32,
    sp: f32,
    seen: u32,
    idx: usize,
    low: bool, // bottom-dweller: swims just above the sand
    gone: bool, // eaten school fish: back with the next pass of the school
    food: Option<Entity>, // the flake this fish is swimming up to
}

#[derive(Resource)]
struct SchoolTarget { pos: Vec3, t: f32, dir: f32, epoch: u32 }

fn rnd(s: &mut u32) -> f32 {
    *s ^= *s << 13;
    *s ^= *s >> 17;
    *s ^= *s << 5;
    (*s & 0xFFFFFF) as f32 / 16777216.0
}

fn thick_for(path: &str) -> &'static [u8] {
    match path {
        "fish/blue_tang.png" => include_bytes!("../assets/fish/blue_tang.thick"),
        "fish/yellow_tang.png" => include_bytes!("../assets/fish/yellow_tang.thick"),
        "fish/clownfish.png" => include_bytes!("../assets/fish/clownfish.thick"),
        "fish/gramma.png" => include_bytes!("../assets/fish/gramma.thick"),
        "fish/flame.png" => include_bytes!("../assets/fish/flame.thick"),
        "fish/bicolor.png" => include_bytes!("../assets/fish/bicolor.thick"),
        "fish/firefish.png" => include_bytes!("../assets/fish/firefish.thick"),
        "fish/mandarin.png" => include_bytes!("../assets/fish/mandarin.thick"),
        "fish/shark.png" => include_bytes!("../assets/fish/shark.thick"),
        _ => include_bytes!("../assets/fish/chromis.thick"),
    }
}

/// Two grids (left and right flank) that meet at the silhouette, giving the fish a closed body.
/// Positions are rewritten each frame.
fn fish_mesh(thick: &[u8]) -> Mesh {
    let n = (NX + 1) * (NY + 1);
    let mut uv = Vec::with_capacity(2 * n);
    let mut col = Vec::with_capacity(2 * n);
    for _ in 0..2 {
        for j in 0..=NY {
            for i in 0..=NX {
                uv.push([i as f32 / NX as f32, j as f32 / NY as f32]);
                // darker toward the outline so the body reads as round
                let t = thick[j * (NX + 1) + i] as f32 / 255.0;
                let c = 0.62 + 0.38 * t.powf(0.5);
                col.push([c, c, c, 1.0]);
            }
        }
    }
    let mut idx = Vec::new();
    let w = (NX + 1) as u32;
    for sheet in 0..2u32 {
        let o = sheet * n as u32;
        for j in 0..NY as u32 {
            for i in 0..NX as u32 {
                let a = o + j * w + i;
                if sheet == 0 {
                    idx.extend_from_slice(&[a, a + 1, a + w, a + 1, a + w + 1, a + w]);
                } else {
                    idx.extend_from_slice(&[a, a + w, a + 1, a + 1, a + w, a + w + 1]);
                }
            }
        }
    }
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0f32, 0.0, 0.0]; 2 * n])
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 2 * n])
        .with_inserted_indices(Indices::U32(idx))
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    cfg: Res<Config>,
) {
    // Render the scene at a fixed lower resolution, then scale it up to the screen.
    let rw: u32 = std::env::var("AQUARIUM_RENDER_W").ok().and_then(|v| v.parse().ok()).unwrap_or(1600);
    let rh = rw * 9 / 16;
    let mut img = Image::new_fill(
        bevy::render::render_resource::Extent3d { width: rw, height: rh, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        &[0, 0, 0, 255],
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.texture_descriptor.usage = bevy::render::render_resource::TextureUsages::TEXTURE_BINDING
        | bevy::render::render_resource::TextureUsages::COPY_DST
        | bevy::render::render_resource::TextureUsages::RENDER_ATTACHMENT;
    let render_img = images.add(img);
    let ui_layer = bevy::camera::visibility::RenderLayers::layer(1);
    commands.spawn((
        Camera3d::default(),
        Camera { clear_color: ClearColorConfig::Custom(Color::BLACK), ..default() },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::Fixed { width: 2.0, height: 2.0 },
            ..OrthographicProjection::default_3d()
        }),
        bevy::core_pipeline::tonemapping::Tonemapping::None,
        Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        ui_layer.clone(),
        IsDefaultUiCamera,
    ));
    let up_mat = mats.add(StandardMaterial {
        base_color_texture: Some(render_img.clone()),
        unlit: true,
        fog_enabled: false,
        ..default()
    });
    commands.insert_resource(UpscaleMat(up_mat.clone()));
    commands.spawn((
        Mesh3d(meshes.add(vignette_grid(32, 18))),
        MeshMaterial3d(up_mat),
        Transform::default(),
        Upscale,
        ui_layer,
    ));
    commands.spawn((
        Camera3d::default(),
        Camera { order: -1, ..default() },
        bevy::camera::RenderTarget::from(render_img.clone()),
        Msaa::Off,
        DistanceFog { color: Color::srgb(0.09, 0.36, 0.50), falloff: FogFalloff::Exponential { density: 0.55 }, ..default() },
        AmbientLight { color: Color::srgb(0.55, 0.75, 1.0), brightness: 250.0, ..default() },
        Transform::from_xyz(0.0, -0.10, 1.1).looking_at(Vec3::new(0.0, -0.02, 0.0), Vec3::Y),
        SceneCam,
    ));
    commands.spawn((
        DirectionalLight { illuminance: 14000.0, ..default() },
        Transform::from_xyz(0.5, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // sand floor
    let sand: Handle<Image> = assets.load_builder().with_settings(|s: &mut bevy::image::ImageLoaderSettings| {
        s.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
            address_mode_u: bevy::image::ImageAddressMode::MirrorRepeat,
            address_mode_v: bevy::image::ImageAddressMode::MirrorRepeat,
            ..default()
        });
    }).load("tex/sand.jpg");
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(2.0, 1.2)))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color_texture: Some(sand),
            base_color: Color::srgb(0.8, 0.95, 1.05),
            uv_transform: bevy::math::Affine2::from_scale(Vec2::new(4.0, 2.5)),
            perceptual_roughness: 1.0,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, FLOOR, -0.3),
    ));
    // fine real grain over the sand: a high-pass of Poly Haven gravel, fading out with distance
    if std::env::var("AQUARIUM_NO_DETAIL").is_err() {
        let detail = repeat_img(&assets, "tex/sand_detail.png");
        let (nx, nz) = (20usize, 20usize);
        let (hx, hz) = (2.0f32, 1.2f32);
        let mut pos = Vec::new();
        let mut uv = Vec::new();
        let mut col = Vec::new();
        for j in 0..=nz {
            for i in 0..=nx {
                let (u, v) = (i as f32 / nx as f32, j as f32 / nz as f32);
                let (x, z) = ((u * 2.0 - 1.0) * hx, (v * 2.0 - 1.0) * hz);
                pos.push([x, 0.0, z]);
                uv.push([u * 5.0, v * 3.0]);
                let zw = -0.3 + z; // world z
                let a = ((zw + 0.95) / 0.75).clamp(0.0, 1.0); // full near the glass, gone far back
                col.push([1.0, 1.0, 1.0, a * a]);
            }
        }
        let w1 = (nx + 1) as u32;
        let mut idx = Vec::new();
        for j in 0..nz as u32 {
            for i in 0..nx as u32 {
                let a = j * w1 + i;
                idx.extend_from_slice(&[a, a + w1, a + 1, a + 1, a + w1, a + w1 + 1]);
            }
        }
        let n = pos.len();
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 1.0, 0.0]; n])
            .with_inserted_indices(Indices::U32(idx));
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color_texture: Some(detail),
                base_color: Color::srgb(0.85, 0.95, 1.0),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            })),
            Transform::from_xyz(0.0, FLOOR + 0.0012, -0.3),
        ));
    }
    // backdrop gradient
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(8.0, 3.2))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color_texture: Some(assets.load("tex/back.png")),
            unlit: true,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.7, -2.2),
    ));
    // caustic shimmer on the sand
    let cmat = mats.add(StandardMaterial {
        base_color_texture: Some(repeat_img(&assets, "tex/caustic.png")),
        base_color: Color::srgb(0.22, 0.34, 0.38),
        uv_transform: bevy::math::Affine2::from_scale(Vec2::new(6.0, 4.0)),
        alpha_mode: AlphaMode::Add,
        unlit: true,
        fog_enabled: false,
        ..default()
    });
    commands.insert_resource(CausticMat(cmat.clone()));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::new(1.5, 0.9)))),
        MeshMaterial3d(cmat),
        Fx,
        Transform::from_xyz(0.0, FLOOR + 0.003, -0.1),
    ));
    // horizon haze band
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(8.0, 0.5))),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color_texture: Some(assets.load("tex/haze.png")),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            fog_enabled: false,
            ..default()
        })),
        Transform::from_xyz(0.0, FLOOR + 0.18, -1.9),
    ));
    // light rays from the surface
    let ray_tex: Handle<Image> = assets.load("tex/ray.png");
    for (i, (x, w, k)) in [(-1.0f32, 0.62f32, 0.9f32), (-0.52, 0.42, 0.55), (-0.08, 0.78, 1.0), (0.38, 0.5, 0.65), (0.92, 0.66, 0.85)].into_iter().enumerate() {
        commands.spawn((
            Mesh3d(meshes.add(Rectangle::new(w, 1.7))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color_texture: Some(ray_tex.clone()),
                base_color: Color::srgb(0.16 * k, 0.26 * k, 0.28 * k),
                alpha_mode: AlphaMode::Add,
                unlit: true,
                fog_enabled: false,
                double_sided: true,
                cull_mode: None,
                ..default()
            })),
            Transform::from_xyz(x, 0.5, -1.3 - 0.1 * i as f32),
            Ray { x, phase: i as f32 * 1.7, k },
            Fx,
        ));
    }
    // rocks: (texture, aspect w/h, width m, x, z, flip, tint)
    let rocks = [
        ("rock/rock_a.png", 1.31, 0.62, -0.28, -0.45, 1.0, 0.95),
        ("rock/rock_b.png", 0.870, 0.28, -0.50, 0.30, 1.0, 1.0),
        ("rock/zoa_mat.png", 1.89, 0.43, 0.12, -0.37, 1.0, 1.0),
        ("rock/coral_teal.png", 0.843, 0.315, 0.30, -0.42, 1.0, 0.95),
        ("rock/rock_c.png", 0.864, 0.36, 0.50, -0.75, 1.0, 0.80),
        ("rock/rock_d.png", 1.192, 0.46, -0.05, -1.05, 1.0, 0.62),
    ];
    let shadow_tex: Handle<Image> = assets.load("tex/shadow.png");
    let shadow_tex: Handle<Image> = assets.load("tex/shadow.png");
    for (path, aspect, w, x, z, flip, k) in rocks {
        let h = w / aspect;
        let is_crab = path.contains("hermit");
        if z < 0.0 || true {
            let mut ec = commands.spawn((
                Mesh3d(meshes.add(Rectangle::new(w * 1.15, w * 0.4))),
                MeshMaterial3d(mats.add(StandardMaterial {
                    base_color_texture: Some(shadow_tex.clone()),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    fog_enabled: false,
                    ..default()
                })),
                Transform::from_xyz(x, FLOOR + 0.004, z + 0.02 + if z > 0.0 { 0.0 } else { 0.0 })
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            ));
            if is_crab { ec.insert(Crab { x0: x - 0.08, amp: 0.12, period: 70.0, roll: false }); }
        }
        let mut ec = commands.spawn((
            Mesh3d(meshes.add(grid_quad(w, h, 22, 16))),
            RockLight { nx: 22, ny: 16, w, h, flip, sway: if path.contains("zoa") { 0.006 } else if path.contains("coral_teal") { 0.009 } else { 0.0 } },
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color_texture: Some(assets.load(path)),
                base_color: Color::srgb(k * 0.85, k * 0.95, k * 1.1),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                fog_enabled: false,
                double_sided: true,
                cull_mode: None,
                ..default()
            })),
            Transform::from_xyz(x, FLOOR + h * 0.5 - 0.03 - if z > 0.0 { 0.0 } else { 0.0 }, z).with_scale(Vec3::new(flip, 1.0, 1.0)),
        ));
        if is_crab { ec.insert(Crab { x0: x - 0.08, amp: 0.12, period: 70.0, roll: true }); }
    }
    // (texture, aspect w/h, body length m, home position, phase, speed)
    let species = [
        ("fish/blue_tang.png", 2.1328, 0.42, Vec3::new(-0.25, 0.10, 0.0), 0.0, 1.0),
        ("fish/yellow_tang.png", 1.750, 0.34, Vec3::new(0.30, -0.12, -0.2), 2.1, 0.8),
        ("fish/clownfish.png", 2.037, 0.20, Vec3::new(-0.45, -0.16, 0.15), 4.0, 1.3),
    ];
    let mut species: Vec<(&str, f32, f32, Vec3, f32, f32, bool, Vec3)> = species.into_iter().map(|(a, b, c, d, e, f)| (a, b, c, d, e, f, false, Vec3::ZERO)).collect();
    species.push(("fish/gramma.png", 2.262, 0.17, Vec3::new(-0.1, -0.05, -0.25), 1.3, 0.9, false, Vec3::ZERO));
    species.push(("fish/flame.png", 2.037, 0.16, Vec3::new(0.45, 0.0, 0.15), 2.9, 0.8, false, Vec3::ZERO));
    // three more kinds
    species.push(("fish/bicolor.png", 1.3763, 0.22, Vec3::new(0.10, 0.20, -0.1), 7.0, 0.9, false, Vec3::ZERO));
    species.push(("fish/firefish.png", 2.735, 0.20, Vec3::new(-0.35, 0.25, 0.1), 8.2, 1.0, false, Vec3::ZERO));
    species.push(("fish/mandarin.png", 1.6726, 0.15, Vec3::new(0.35, -0.2, 0.1), 9.1, 0.6, false, Vec3::ZERO));
    // a second (slightly smaller) fish of each kind, different start and pace
    species.push(("fish/blue_tang.png", 2.1328, 0.36, Vec3::new(0.55, 0.20, -0.25), 5.2, 0.9, false, Vec3::ZERO));
    species.push(("fish/yellow_tang.png", 1.750, 0.29, Vec3::new(-0.55, 0.15, 0.1), 3.3, 0.9, false, Vec3::ZERO));
    species.push(("fish/clownfish.png", 2.037, 0.17, Vec3::new(0.2, -0.1, -0.2), 6.1, 1.1, false, Vec3::ZERO));
    species.push(("fish/gramma.png", 2.262, 0.15, Vec3::new(0.6, 0.1, -0.1), 4.4, 0.8, false, Vec3::ZERO));
    species.push(("fish/flame.png", 2.037, 0.14, Vec3::new(-0.3, 0.2, 0.12), 0.9, 0.9, false, Vec3::ZERO));
    for i in 0..7 {
        let f = i as f32;
        species.push(("fish/chromis.png", 3.235, 0.075, Vec3::new(0.0 + 0.04 * f, 0.1 + 0.03 * (f % 3.0), -0.2), 0.7 * f, 1.0 + 0.04 * f, true, Vec3::new(0.14 * ((f * 1.7).sin()), 0.07 * ((f * 2.3).cos()), 0.10 * ((f * 3.1).sin()))));
    }
    {
        let (mut ind, mut sch) = (0, 0);
        let _ = &mut ind;
        species.retain(|sp| if sp.6 { sch += 1; sch <= cfg.school } else { true });
    }
    spawn_grass(&mut commands, &mut meshes, &mut mats, &assets, cfg.grass);
    spawn_bubbles(&mut commands, &mut meshes, &mut mats, &assets, cfg.bubbles);
    spawn_motes(&mut commands, &mut meshes, &mut mats, &assets, cfg.motes);
    commands.insert_resource(SchoolTarget { pos: Vec3::new(0.0, 0.1, -0.2), t: 0.0, dir: 1.0, epoch: 0 });
    // every fish keeps its own depth lane: no depth changes (no size pulsing) and no fish pass through each other
    let lanes = [3, 7, 1, 9, 5, 0, 8, 12, 2, 6, 4, 10, 11];
    let mut lane_i = 0;
    let mut ind_i = 0usize;
    for (path, aspect, len, home, phase, speed, school, offset) in species {
        let home = if school { home } else {
            let z = -0.30 + 0.44 * lanes[lane_i % lanes.len()] as f32 / 13.0;
            lane_i += 1;
            Vec3::new(home.x, home.y, z)
        };
        if !school { ind_i = lane_i - 1; }
        let off_now = !school && ind_i >= cfg.fish; // starts out of sight until it is added
        let mat = mats.add(StandardMaterial {
            base_color_texture: Some(assets.load(path)),
            alpha_mode: AlphaMode::Mask(0.5),
            double_sided: true,
            cull_mode: None,
            unlit: true,
            fog_enabled: false,
            base_color: Color::srgb(0.92, 0.97, 1.0),
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(fish_mesh(thick_for(path)))),
            MeshMaterial3d(mat),
            Transform::from_translation(if off_now { Vec3::new(-3.0, home.y, home.z) } else { home }),
            if off_now { Visibility::Hidden } else { Visibility::Inherited },
            Fish { len, height: len / aspect, depth: len * 0.07, phase, home, speed, target: home, wait: if off_now { 1.0 } else { 0.0 }, yaw: 0.0, vel: 0.0, rng: 2463534242u32.wrapping_add((phase * 1000.0) as u32), dart: 0.0, school, offset, thick: thick_for(path), dir: if school { 1.0 } else if home.x < 0.0 { 1.0 } else { -1.0 }, hover: 0.0, vy: 0.0, pitch: 0.0, sp: if school { 0.0 } else { 0.10 + 0.05 * speed }, seen: 0, idx: ind_i, low: path.contains("mandarin"), gone: false, food: None },
        ));
    }
    // the occasional visitor: a whitetip reef shark, hidden until its turn comes
    {
        let (len, aspect) = (0.95f32, 2.6786f32);
        commands.spawn((
            Mesh3d(meshes.add(fish_mesh(thick_for("fish/shark.png")))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color_texture: Some(assets.load("fish/shark.png")),
                alpha_mode: AlphaMode::Mask(0.5),
                double_sided: true,
                cull_mode: None,
                unlit: true,
                fog_enabled: false,
                base_color: Color::srgb(0.92, 0.97, 1.0),
                ..default()
            })),
            Transform::from_xyz(-3.0, 0.0, 0.0),
            Visibility::Hidden,
            Fish { len, height: len / aspect, depth: len * 0.07, phase: 0.0, home: Vec3::ZERO, speed: 0.45, target: Vec3::ZERO, wait: 0.0, yaw: 0.0, vel: 0.55, rng: 99991, dart: 0.0, school: false, offset: Vec3::ZERO, thick: thick_for("fish/shark.png"), dir: 1.0, hover: 0.0, vy: 0.0, pitch: 0.0, sp: 0.0, seen: 0, idx: usize::MAX, low: false, gone: false, food: None },
            Shark { active: false, prey: None, eats: 0 },
        ));
    }
}

fn swim(
    time: Res<Time>,
    cfg: Res<Config>,
    day: Res<DayCycle>,
    mut meshes: ResMut<Assets<Mesh>>,
    q: Query<(&Mesh3d, &Fish, &Transform, &Visibility)>,
) {
    for (m, f, tf, vis) in &q {
        if *vis == Visibility::Hidden { continue; }
        let t = time.elapsed_secs() * cfg.speed * f.speed + f.phase;
        let Some(mut mesh) = meshes.get_mut(&m.0) else { continue };
        let n = (NX + 1) * (NY + 1);
        let mut pos = vec![[0.0f32; 3]; 2 * n];
        let mut col = vec![[1.0f32, 1.0, 1.0, 1.0]; 2 * n];
        let lt = time.elapsed_secs() * cfg.speed;
        let dep = f.height * 0.17;
        // the depth-wise wave and fin motion only show as tearing when seen end-on: calm them there
        let sd = f.yaw.cos().abs();
        let calm = 0.1 + 0.9 * sd * sd * sd;
        // seen from the side of the tank the far flank would show up as a second, shifted fish: slim down with viewing angle
        let tn = tf.translation.x.abs() / (1.1 - tf.translation.z).max(0.3);
        let slim = 1.0 / (1.0 + 4.0 * tn * tn.max(0.25));
        for j in 0..=NY {
            let v = j as f32 / NY as f32;
            let y0 = (0.5 - v) * f.height;
            for i in 0..=NX {
                let u = i as f32 / NX as f32;
                let x = (u - 0.5) * f.len * (0.3 + 0.7 * sd);
                let from_head = 1.0 - u; // 0 head .. 1 tail
                let amp = (0.002 + 0.06 * from_head * from_head * f.len) * (0.3 + 0.7 * f.vel);
                let z_bend = amp * (6.0 * from_head - 5.0 * t).sin();
                let y = y0 + 0.9 * z_bend * (0.4 + from_head);
                let k = j * (NX + 1) + i;
                let th = dep * (f.thick[k] as f32 / 255.0) * (1.0 + 0.8 * (1.0 - sd)) * slim;
                // fin flutter: ripple along the dorsal/anal edges, pectoral flap near the head
                let edge = (2.0 * v - 1.0).abs().powi(3);
                let ripple = f.len * 0.012 * (0.6 + f.vel) * edge * (18.0 * u + 7.0 * t).sin();
                let pw = (-(((u - 0.66) / 0.09).powi(2) + ((v - 0.58) / 0.15).powi(2))).exp();
                let flap = f.len * 0.045 * pw * (0.5 + 0.5 * (6.0 * t).sin());
                let zb = z_bend * calm;
                let (ripple, flap) = (ripple * calm, flap * calm);
                pos[k] = [x, y, zb + th + ripple + flap];
                pos[n + k] = [x, y, zb - th + ripple - flap];
                // sunlight ripples across the body, as on the real thing
                let tt = f.thick[k] as f32 / 255.0;
                let shade = (0.62 + 0.38 * tt.powf(0.5)) * light_mul(Vec3::new(tf.translation.x + x * f.dir, tf.translation.y + y, tf.translation.z), lt, day.d);
                col[k] = [shade, shade, shade, 1.0];
                col[n + k] = [shade, shade, shade, 1.0];
            }
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    }
}

/// Dev aid: SHOT=/path.png captures only this window after 4s, then exits.
fn auto_shot(mut commands: Commands, time: Res<Time>, mut done: Local<bool>, mut exit: MessageWriter<AppExit>) {
    let Ok(path) = std::env::var("SHOT") else { return };
    let at: f32 = std::env::var("SHOT_T").ok().and_then(|v| v.parse().ok()).unwrap_or(4.0);
    if !*done && time.elapsed_secs() > at {
        *done = true;
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
    } else if *done && time.elapsed_secs() > at + 2.0 {
        exit.write(AppExit::Success);
    }
}

/// Fish always face the way they swim, never turn on screen: they cross the tank, leave
/// through a side, wait out of sight, and come back in from either side.
fn drift(time: Res<Time>, cfg: Res<Config>, day: Res<DayCycle>, st: Res<SchoolTarget>, mut flakes: Query<(Entity, &Transform, &mut Flake), Without<Fish>>, mut commands: Commands, mut q: Query<(&mut Transform, &mut Fish, &mut Visibility), Without<Shark>>) {
    let dt = time.delta_secs().min(0.1);
    let trace = std::env::var("AQUARIUM_TRACE").is_ok();
    let showcase = std::env::var("AQUARIUM_SHOWCASE").is_ok();
    for (mut tf, mut f, mut vis) in &mut q {
        if showcase && !f.school {
            // debug: every species in a still grid, for inspecting the cutouts
            tf.translation = Vec3::new(-0.6 + 0.4 * (f.idx % 4) as f32, 0.25 - 0.16 * (f.idx / 4) as f32, 0.0);
            tf.rotation = Quat::IDENTITY;
            f.dir = 1.0;
            f.yaw = 0.0;
            *vis = Visibility::Inherited;
            continue;
        }
        let lim = 1.15 + f.len * 0.5;
        let night = 1.0 - day.d;
        // at night the fish slow down, rest more often and settle lower in the tank
        let cruise = (0.10 + 0.05 * f.speed) * cfg.speed * (1.0 - 0.5 * night);
        let band = 0.30 * (1.0 - 0.6 * night);
        let (ybase, yrange) = if f.low { (0.055, 0.05) } else { (0.20, band) };
        if f.school && f.gone {
            // eaten: stays away until the school next comes round
            if f.seen != st.epoch { f.gone = false; *vis = Visibility::Inherited; } else { *vis = Visibility::Hidden; continue; }
        }
        if f.school {
            f.target = st.pos + f.offset;
            if f.seen != st.epoch {
                // the school has re-entered (possibly on the same side): jump with it, off-screen, and face its way
                tf.translation = f.target;
                f.dir = st.dir;
                f.seen = st.epoch;
            }
            let to = f.target - tf.translation;
            f.vel += (0.8 - f.vel) * (dt * 1.2);
            let mut step = to * (dt * 1.5).min(1.0);
            // never drift backwards relative to the way the fish faces
            if step.x * f.dir < 0.03 * dt { step.x = f.dir * 0.03 * dt; }
            tf.translation += step;
        } else if f.wait > 0.0 {
            f.wait -= dt;
            if f.wait <= 0.0 && f.idx >= cfg.fish {
                f.wait = 1.0; // not in the tank at the moment
            } else if f.wait <= 0.0 {
                *vis = Visibility::Inherited;
                // come back in from a random side
                f.dir = if rnd(&mut f.rng) < 0.5 { 1.0 } else { -1.0 };
                let r = [rnd(&mut f.rng), rnd(&mut f.rng), rnd(&mut f.rng)];
                let z = f.home.z;
                f.target = Vec3::new(0.0, FLOOR + ybase + yrange * r[1] * r[1], z);
                tf.translation = Vec3::new(-f.dir * lim, f.target.y, z);
                f.sp = cruise;
            }
        } else {
            // speed is eased, never switched: slowing to a hover takes a few seconds
            let mut rate = 0.5;
            let want_sp;
            if f.hover > 0.0 {
                f.hover -= dt;
                want_sp = 0.0;
            } else {
                if tf.translation.x.abs() < 0.8 && rnd(&mut f.rng) < 0.04 * (1.0 + 3.0 * night) * dt {
                    f.hover = 5.0 + 6.0 * rnd(&mut f.rng);
                }
                if rnd(&mut f.rng) < 0.02 * dt {
                    f.dart = 1.5;
                }
                if f.dart > 0.0 { rate = 1.0; }
                want_sp = if f.dart > 0.0 { cruise * 2.2 } else { cruise };
                f.dart = (f.dart - dt).max(0.0);
            }
            let mut want_sp = want_sp;
            let mut feed_vy: Option<f32> = None;
            if f.food.is_none() && !f.low && f.idx < cfg.fish {
                // any food in this fish's own depth lane and ahead of it: go for the nearest
                let mut best: Option<(Entity, f32)> = None;
                for (e, ft, fl) in &flakes {
                    if fl.taken || fl.landed > 0.0 || (ft.translation.z - f.home.z).abs() > 0.02 || ft.translation.y < FLOOR + 0.05 { continue; }
                    let dx = (ft.translation.x - tf.translation.x) * f.dir;
                    if dx > 0.03 && dx < 1.3 && best.map_or(true, |b| dx < b.1) { best = Some((e, dx)); }
                }
                if let Some((e, _)) = best {
                    if let Ok((_, _, mut fl)) = flakes.get_mut(e) { fl.taken = true; }
                    f.food = Some(e);
                }
            }
            if let Some(e) = f.food {
                match flakes.get_mut(e) {
                    Ok((_, ft, mut fl)) => {
                        let dx = (ft.translation.x - tf.translation.x) * f.dir;
                        let dyf = ft.translation.y - tf.translation.y;
                        if dx < -0.05 || fl.landed > 0.0 {
                            fl.taken = false;
                            f.food = None; // swam past it
                        } else if dx < 0.05 && dyf.abs() < 0.05 {
                            commands.entity(e).despawn(); // gulp
                            println!("ate food");
                            f.food = None;
                        } else {
                            f.hover = 0.0;
                            f.dart = 0.0;
                            rate = 1.0;
                            want_sp = (dx * 1.6).clamp(0.05, cruise * 2.0);
                            feed_vy = Some((dyf * 1.8).clamp(-0.24, 0.24));
                        }
                    }
                    Err(_) => f.food = None,
                }
            }
            f.sp += (want_sp - f.sp) * (dt * rate).min(1.0);
            f.vel = f.sp / 0.15;
            tf.translation.x += f.dir * f.sp * dt;
            // drift gently up/down toward a wandering height
            let dy = f.target.y - tf.translation.y;
            // fish change depth by swimming at a slope, not by sliding straight up or down: the climb rate is tied to forward speed
            let (vy_want, vrate) = match feed_vy {
                Some(v) => (v, 2.0),
                None => { let vmax = f.sp * 0.3; ((dy * 0.5).clamp(-vmax, vmax), 0.6) }
            };
            f.vy += (vy_want - f.vy) * (dt * vrate).min(1.0);
            tf.translation.y += f.vy * dt;
            if dy.abs() < 0.02 {
                f.target.y = FLOOR + ybase + yrange * rnd(&mut f.rng).powi(2);
            }
            if tf.translation.x.abs() > lim {
                f.wait = 0.5 + 3.5 * rnd(&mut f.rng);
                f.food = None;
                if f.idx >= cfg.fish { *vis = Visibility::Hidden; } // removed fish leave and stay away
            }
        }
        f.yaw = if f.dir > 0.0 { 0.0 } else { std::f32::consts::PI };
        let feeding = f.food.is_some();
        let pmax = if feeding { 0.9 } else { 0.3 };
        let want = if f.school { 0.0 } else { (f.vy / f.sp.max(if feeding { 0.10 } else { 0.06 })).atan().clamp(-pmax, pmax) };
        f.pitch += (want - f.pitch) * (dt * if feeding { 2.5 } else { 1.2 });
        let pitch = f.pitch * f.dir;
        tf.rotation = Quat::from_euler(EulerRot::YXZ, f.yaw, 0.0, pitch);
        if trace && ((!f.school && (f.len - 0.42).abs() < 1e-4) || (f.school && (f.phase - 2.1).abs() < 0.01)) && time.elapsed_secs() % 1.0 < dt {
            println!("TRACE t={:.0} {} x={:.3} dir={} sp={:.3} hover={:.1} wait={:.1}", time.elapsed_secs(), if f.school { "school" } else { "tang  " }, tf.translation.x, f.dir, f.sp, f.hover, f.wait);
        }
        if let Some(d) = std::env::var("AQUARIUM_YAW").ok().and_then(|v| v.parse::<f32>().ok()) {
            f.yaw = d.to_radians();
            tf.rotation = Quat::from_rotation_y(f.yaw);
        }
    }
}

/// Slow ray sway and caustic scroll.
fn animate_light(
    time: Res<Time>,
    cm: Res<CausticMat>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut rays: Query<(&mut Transform, &Ray)>,
) {
    let t = time.elapsed_secs();
    for (mut tf, r) in &mut rays {
        tf.translation.x = r.x + 0.05 * (t * 0.15 + r.phase).sin();
        tf.rotation = Quat::from_rotation_z(0.18 + 0.05 * (t * 0.2 + r.phase).sin());
    }
    if let Some(mut m) = mats.get_mut(&cm.0) {
        m.uv_transform = bevy::math::Affine2::from_scale_angle_translation(
            Vec2::new(6.0, 4.0), 0.0, Vec2::new(t * 0.012, t * 0.008 + 0.05 * (t * 0.4).sin()));
    }
}

/// Gently push fish apart in depth when they overlap, so they do not pass through each other.
fn separate(time: Res<Time>, mut q: Query<&mut Transform, With<Fish>>) {
    let dt = time.delta_secs().min(0.1);
    let mut it = q.iter_combinations_mut::<2>();
    while let Some([mut a, mut b]) = it.fetch_next() {
        let d = a.translation - b.translation;
        if d.x.abs() < 0.35 && d.y.abs() < 0.2 && d.z.abs() < 0.16 {
            let push = 0.25 * dt * if d.z >= 0.0 { 1.0 } else { -1.0 };
            a.translation.z += push;
            b.translation.z -= push;
            a.translation.y += 0.1 * dt * d.y.signum();
            b.translation.y -= 0.1 * dt * d.y.signum();
        }
    }
}

/// The school's leader swims across the tank, leaves, waits out of sight, returns.
fn school_target(time: Res<Time>, cfg: Res<Config>, mut st: ResMut<SchoolTarget>, mut seed: Local<u32>) {
    if *seed == 0 { *seed = 88172645; }
    let dt = time.delta_secs().min(0.1);
    let lim = 1.85;
    if st.t > 0.0 {
        st.t -= dt;
        if st.t <= 0.0 {
            let r = [rnd(&mut seed), rnd(&mut seed), rnd(&mut seed)];
            st.dir = if r[0] < 0.5 { 1.0 } else { -1.0 };
            st.pos = Vec3::new(-st.dir * lim, FLOOR + 0.22 + 0.28 * r[1], if r[2] < 0.5 { -0.28 } else { 0.10 });
            st.epoch += 1;
        }
    } else {
        st.pos.x += st.dir * 0.14 * cfg.speed * dt;
        st.pos.y += 0.02 * (time.elapsed_secs() * 0.4).sin() * dt;
        if st.pos.x.abs() > lim {
            st.t = 2.0 + 4.0 * rnd(&mut seed);
        }
    }
}

/// Prints the average frame rate every 10 s when AQUARIUM_FPS is set.
fn fps_log(time: Res<Time>, mut acc: Local<(f32, u32)>) {
    if std::env::var("AQUARIUM_FPS").is_err() { return; }
    acc.0 += time.delta_secs();
    acc.1 += 1;
    if acc.0 >= 10.0 {
        println!("fps {:.1}", acc.1 as f32 / acc.0);
        *acc = (0.0, 0);
    }
}

/// If the frame rate stays low, switch off the optional light effects (rays, caustics).
fn auto_quality(
    time: Res<Time>,
    mut acc: Local<(f32, u32, bool)>,
    mut fx: Query<&mut Visibility, With<Fx>>,
) {
    if acc.2 || time.elapsed_secs() < 10.0 || std::env::var("AQUARIUM_AUTOQ").is_err() { return; }
    acc.0 += time.delta_secs();
    acc.1 += 1;
    if acc.0 >= 8.0 {
        let fps = acc.1 as f32 / acc.0;
        if fps < 40.0 && std::env::var("AQUARIUM_NOVSYNC").is_err() {
            for mut v in &mut fx { *v = Visibility::Hidden; }
            acc.2 = true;
            println!("quality: low fps ({fps:.0}), light effects disabled");
        }
        *acc = (0.0, 0, acc.2);
    }
}

/// Keep the 16:9 picture undistorted on any window shape (letterbox).
fn fit_upscale(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut q: Query<&mut Transform, Or<(With<Upscale>, With<Vignette>)>>,
) {
    let Ok(w) = windows.single() else { return };
    let a = w.width() / w.height().max(1.0);
    let c = 16.0 / 9.0;
    for mut tf in &mut q {
        tf.scale = if a >= c { Vec3::new(c / a, 1.0, 1.0) } else { Vec3::new(1.0, a / c, 1.0) };
    }
}

// ---------------------------------------------------------------- seagrass

#[derive(Component)]
struct Grass {
    rest: Vec<[f32; 3]>,
    w: Vec<f32>, // 0 at the root .. 1 at the tip
    phase: f32,
}

const GRASS_SEGS: usize = 12;

fn hash(i: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E3779B1) ^ 0x85EBCA6B;
    x ^= x >> 15; x = x.wrapping_mul(0x2C1B3C6D); x ^= x >> 12; x = x.wrapping_mul(0x297A2D39); x ^= x >> 15;
    (x & 0xFFFFFF) as f32 / 16777216.0
}

/// Clumps of textured ribbon blades; one mesh per clump, bent every frame by `sway_grass`.
fn spawn_grass(commands: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>, assets: &AssetServer, clumps: usize) {
    if clumps == 0 { return; }
    let mat = mats.add(StandardMaterial {
        base_color_texture: Some(assets.load("tex/blade.png")),
        base_color: Color::srgb(0.85, 0.95, 1.0),
        unlit: true,
        fog_enabled: false,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    for c in 0..clumps as u32 {
        let k = c * 97 + 13;
        let cx = -1.0 + 2.0 * hash(k);
        let front = hash(k + 1) < 0.15;
        let cz = if front { 0.18 + 0.15 * hash(k + 2) } else { -0.66 + 0.30 * hash(k + 2) };
        let scale = if front { 0.55 } else { 1.0 };
        let nb = 6 + (hash(k + 3) * 6.0) as u32;
        let haze = ((-cz - 0.30) / 0.4).clamp(0.0, 1.0);
        let mut pos = Vec::new();
        let mut col = Vec::new();
        let mut uv = Vec::new();
        let mut wgt = Vec::new();
        let mut idx = Vec::new();
        for b in 0..nb {
            let kb = k * 31 + b * 7;
            let bx = (hash(kb) - 0.5) * 0.08;
            let bz = (hash(kb + 1) - 0.5) * 0.05;
            let ang = hash(kb + 2) * std::f32::consts::PI;
            let (wx, wz) = (ang.cos(), ang.sin());
            let h = (0.14 + 0.22 * hash(kb + 3)) * scale;
            let wid = (0.0068 + 0.0050 * hash(kb + 4)) * scale.sqrt();
            let lean = (hash(kb + 5) - 0.5) * 0.10 * scale;
            // tips arch over to one side, like real blades
            let arch = (hash(kb + 7) - 0.5) * 0.16 * scale;
            let curl = (hash(kb + 8) - 0.5) * 6.0;
            let shade = 0.82 + 0.30 * hash(kb + 6);
            let base = pos.len() as u32;
            for j in 0..=GRASS_SEGS {
                let f = j as f32 / GRASS_SEGS as f32;
                // rounded, slightly twisting ribbon that narrows only near the tip
                let half = wid * (1.0 - f.powf(3.5) * 0.85) * (0.8 + 0.2 * (f * 9.0 + curl).sin());
                let y = FLOOR - 0.004 + h * (f - 0.12 * f * f * (arch.abs() * 6.0));
                let off = lean * f * f + arch * f.powi(3);
                // the far clumps get bluer and darker, as if seen through more water
                let vc = [shade * (1.0 - 0.55 * haze), shade * (1.0 - 0.25 * haze), shade * (1.0 - 0.05 * haze) + 0.05 * haze, 1.0];
                for (side, u) in [(-1.0f32, 0.0f32), (0.0, 0.5), (1.0, 1.0)] {
                    pos.push([cx + bx + off * wx + side * half * wx, y, cz + bz + off * wz + side * half * wz]);
                    col.push(vc);
                    uv.push([u, f]);
                    wgt.push(f);
                }
                if j > 0 {
                    let a = base + 3 * (j as u32 - 1);
                    idx.extend_from_slice(&[a, a + 1, a + 3, a + 1, a + 4, a + 3, a + 1, a + 2, a + 4, a + 2, a + 5, a + 4]);
                }
            }
        }
        let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos.clone())
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
            .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; pos.len()])
            .with_inserted_indices(Indices::U32(idx));
        commands.spawn((
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(mat.clone()),
            Transform::default(),
            bevy::camera::visibility::NoFrustumCulling,
            Grass { rest: pos, w: wgt, phase: hash(k + 9) * 6.28 },
        ));
    }
}

/// Slow swaying in a gentle current; tips move most, roots stay put.
fn sway_grass(time: Res<Time>, cfg: Res<Config>, mut meshes: ResMut<Assets<Mesh>>, q: Query<(&Mesh3d, &Grass)>) {
    let t = time.elapsed_secs() * cfg.speed;
    for (m, g) in &q {
        let Some(mut mesh) = meshes.get_mut(&m.0) else { continue };
        let mut pos = Vec::with_capacity(g.rest.len());
        for (p, w) in g.rest.iter().zip(&g.w) {
            let bend = w * w;
            let current = (t * 0.7 + g.phase + p[0] * 2.0).sin() * 0.030 + (t * 1.9 + g.phase * 1.7 + p[0] * 5.0).sin() * 0.008;
            let dz = (t * 0.5 + g.phase * 0.6).sin() * 0.012;
            pos.push([p[0] + current * bend, p[1] - 0.004 * bend * (1.0 + current * 10.0).abs(), p[2] + dz * bend]);
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    }
}

fn hide_fx(cfg: Res<Config>, mut fx: Query<&mut Visibility, With<Fx>>) {
    if cfg.no_effects {
        for mut v in &mut fx { *v = Visibility::Hidden; }
    }
}

// ---------------------------------------------------------------- bubbles

#[derive(Component)]
struct Bubble { id: u32, stream: usize, offset: f32, period: f32, size: f32 }

/// (x, z) of the air-stone streams; bubbles start just under the sand and rise out of the top.
const STREAMS: [(f32, f32); 3] = [(0.62, -0.52), (-0.80, -0.46), (0.05, -0.70)];
const BUBBLE_TOP: f32 = 0.55;

fn spawn_bubbles(commands: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>, assets: &AssetServer, n: usize) {
    if n == 0 { return; }
    let mat = mats.add(StandardMaterial {
        base_color_texture: Some(assets.load("tex/bubble.png")),
        base_color: Color::srgb(1.0, 1.0, 1.0),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        fog_enabled: false,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    for i in 0..n as u32 {
        let stream = (i % 3) as usize;
        let rank = i / 3;
        let per_stream = (n as u32 / 3).max(1);
        let speed = 0.13 + 0.05 * hash(i * 5 + 1);
        let period = (BUBBLE_TOP - FLOOR + 0.03) / speed;
        // evenly staggered, with a little jitter so the stream is not regular
        let offset = (rank as f32 + 0.6 * hash(i * 5 + 2)) / per_stream as f32 * period;
        commands.spawn((
            Mesh3d(quad.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(STREAMS[stream].0, FLOOR, STREAMS[stream].1),
            bevy::camera::visibility::NoFrustumCulling,
            Bubble { id: i, stream, offset, period, size: 0.011 + 0.010 * hash(i * 5 + 3) },
        ));
    }
}

/// Bubbles rise, widen a little, wobble side to side and speed up as they grow.
fn animate_bubbles(time: Res<Time>, cfg: Res<Config>, mut q: Query<(&mut Transform, &Bubble)>) {
    let t = time.elapsed_secs() * cfg.speed;
    for (mut tf, b) in &mut q {
        let a = t + b.offset;
        let cycle = (a / b.period).floor();
        let u = (a / b.period) - cycle; // 0 at the sand .. 1 at the top
        let cid = b.id.wrapping_mul(2654435761).wrapping_add(cycle as u32);
        let vary = 0.65 + 0.7 * hash(cid);
        let (sx, sz) = STREAMS[b.stream];
        let y = FLOOR - 0.02 + (BUBBLE_TOP - FLOOR + 0.03) * (u + 0.25 * u * u) / 1.25;
        let spread = 0.012 + 0.05 * u; // the stream fans out as it rises
        let x = sx + spread * (hash(cid + 7) - 0.5) * 2.0 + 0.008 * (u * 14.0 + b.id as f32).sin() * (0.3 + u);
        tf.translation = Vec3::new(x, y, sz + 0.02 * (hash(cid + 11) - 0.5));
        tf.scale = Vec3::splat(b.size * vary * (1.0 + 0.5 * u));
    }
}

// ---------------------------------------------------------------- caustic light on fish and rocks

/// Brightness multiplier at a point in the tank: sunlight focused by the surface ripples,
/// as a moving net of bright lines that fades with depth.
fn light_mul(p: Vec3, t: f32, d: f32) -> f32 {
    let (x, y, z) = (p.x * 11.0, p.y * 11.0, p.z * 7.0);
    let a = (x * 1.3 + y * 0.7 + z + t * 0.55).sin();
    let b = (-x * 0.8 + y * 1.4 - z * 0.6 + t * 0.43 + 1.7).sin();
    let c = (x * 0.5 - y * 1.1 + z * 1.2 - t * 0.37 + 4.1).sin();
    let ridge = (1.0 - ((a + b + c) / 3.0).abs()).powi(6);
    let depth = ((p.y - FLOOR) / 0.55 + 0.35).clamp(0.3, 1.0);
    0.86 + 0.55 * ridge * depth * d
}

#[derive(Component)]
struct RockLight { nx: usize, ny: usize, w: f32, h: f32, flip: f32, sway: f32 }

fn grid_quad(w: f32, h: f32, nx: usize, ny: usize) -> Mesh {
    let mut pos = Vec::new();
    let mut uv = Vec::new();
    for j in 0..=ny {
        for i in 0..=nx {
            let (u, v) = (i as f32 / nx as f32, j as f32 / ny as f32);
            pos.push([(u - 0.5) * w, (0.5 - v) * h, 0.0]);
            uv.push([u, v]);
        }
    }
    let w1 = (nx + 1) as u32;
    let mut idx = Vec::new();
    for j in 0..ny as u32 {
        for i in 0..nx as u32 {
            let a = j * w1 + i;
            idx.extend_from_slice(&[a, a + w1, a + 1, a + 1, a + w1, a + w1 + 1]);
        }
    }
    let n = pos.len();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[1.0f32, 1.0, 1.0, 1.0]; n])
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 0.0, 1.0]; n])
        .with_inserted_indices(Indices::U32(idx))
}

fn light_rocks(time: Res<Time>, cfg: Res<Config>, day: Res<DayCycle>, mut meshes: ResMut<Assets<Mesh>>, q: Query<(&Mesh3d, &Transform, &RockLight)>) {
    let t = time.elapsed_secs() * cfg.speed;
    for (m, tf, r) in &q {
        let Some(mut mesh) = meshes.get_mut(&m.0) else { continue };
        let mut col = Vec::with_capacity((r.nx + 1) * (r.ny + 1));
        if r.sway > 0.0 {
            // soft corals and polyps sway in the current: the top bends most, the base stays put
            let mut pos = Vec::with_capacity((r.nx + 1) * (r.ny + 1));
            for j in 0..=r.ny {
                let v = j as f32 / r.ny as f32;
                let weight = (1.0 - v).powf(1.6);
                for i in 0..=r.nx {
                    let u = i as f32 / r.nx as f32;
                    let ph = tf.translation.x * 7.0;
                    let dx = r.sway * weight * ((t * 0.8 + ph + u * 5.0).sin() + 0.45 * (t * 1.9 + ph * 1.3 + v * 9.0).sin());
                    pos.push([(u - 0.5) * r.w + dx, (0.5 - v) * r.h, 0.0]);
                }
            }
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
        }
        for j in 0..=r.ny {
            let ly = (0.5 - j as f32 / r.ny as f32) * r.h;
            for i in 0..=r.nx {
                let lx = (i as f32 / r.nx as f32 - 0.5) * r.w * r.flip;
                let m = light_mul(Vec3::new(tf.translation.x + lx, tf.translation.y + ly, tf.translation.z), t, day.d);
                col.push([m, m, m, 1.0]);
            }
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, col);
    }
}

// ---------------------------------------------------------------- on-screen display

#[derive(Resource)]
struct Osd { hud_until: f32, help_until: f32, msg: String }

impl Default for Osd {
    fn default() -> Self {
        // AQUARIUM_OSD=hud|help|both keeps the display on (for screenshots)
        let m = std::env::var("AQUARIUM_OSD").unwrap_or_default();
        let on = |k: &str| if m == k || m == "both" { 1.0e9 } else { 0.0 };
        Osd { hud_until: on("hud"), help_until: on("help"), msg: String::new() }
    }
}

impl Osd {
    fn show_hud(&mut self, now: f32, msg: &str) {
        self.hud_until = now + 3.0;
        self.msg = msg.to_string();
    }
}

#[derive(Component, Clone, Copy, PartialEq)]
enum Role {
    HudPanel, HudLabel, BarBack, Msg,
    FishFill, FishVal, SpeedFill, SpeedVal,
    HelpPanel, HelpText,
}

fn setup_osd(mut commands: Commands) {
    let vh = |v: f32| Val::Vh(v);
    let txt = |size: f32| TextFont { font_size: FontSize::Vh(size), ..default() };
    let none = Color::srgba(1.0, 1.0, 1.0, 0.0);
    // status panel, bottom centre
    commands.spawn(Node {
        position_type: PositionType::Absolute,
        bottom: vh(4.0),
        width: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        ..default()
    }).with_children(|root| {
        root.spawn((
            Role::HudPanel,
            Node { flex_direction: FlexDirection::Column, padding: UiRect::all(vh(1.6)), row_gap: vh(0.9), border_radius: BorderRadius::all(vh(1.2)), ..default() },
            BackgroundColor(none),
        )).with_children(|panel| {
            for (label, fill, val) in [("Fish", Role::FishFill, Role::FishVal), ("Speed", Role::SpeedFill, Role::SpeedVal)] {
                panel.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: vh(1.4), ..default() })
                    .with_children(|row| {
                        row.spawn((Role::HudLabel, Text::new(label), txt(2.1), TextColor(none), Node { width: vh(8.0), ..default() }));
                        row.spawn((Role::BarBack, Node { width: vh(24.0), height: vh(1.1), border_radius: BorderRadius::all(vh(0.55)), ..default() }, BackgroundColor(none)))
                            .with_children(|bar| {
                                bar.spawn((fill, Node { width: Val::Percent(50.0), height: Val::Percent(100.0), border_radius: BorderRadius::all(vh(0.55)), ..default() }, BackgroundColor(none)));
                            });
                        row.spawn((val, Text::new(""), txt(2.1), TextColor(none), TextLayout::no_wrap(), Node { width: vh(19.0), ..default() }));
                    });
            }
            panel.spawn((Role::Msg, Text::new(""), txt(1.8), TextColor(none)));
        });
    });
    // help panel, centre
    commands.spawn(Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }).with_children(|root| {
        root.spawn((
            Role::HelpPanel,
            Node { padding: UiRect::all(vh(3.0)), border_radius: BorderRadius::all(vh(1.5)), ..default() },
            BackgroundColor(none),
        )).with_children(|panel| {
            panel.spawn((
                Role::HelpText,
                Text::new("Keys\n\n  F / D           add / remove a fish\n  Up / Right      faster\n  Down / Left     slower\n  P               pause / resume\n  E               feed the fish\n  K               call the shark (Shift+K: on / off)\n  S               save a screenshot\n  N               day/night on / off\n  Shift+N         jump ahead in the day\n  F11             fullscreen / window\n  ? or H          this help\n  Esc or Q        quit\n\nSettings are remembered for next time."),
                txt(2.3),
                TextColor(none),
            ));
        });
    });
}

fn update_osd(
    real: Res<Time<Real>>,
    vtime: Res<Time<Virtual>>,
    cfg: Res<Config>,
    osd: Res<Osd>,
    mut q: Query<(&Role, Option<&mut BackgroundColor>, Option<&mut TextColor>, Option<&mut Node>, Option<&mut Text>)>,
) {
    let now = real.elapsed_secs();
    let hud = if vtime.is_paused() { 1.0 } else { ((osd.hud_until - now) / 0.7).clamp(0.0, 1.0) };
    let help = ((osd.help_until - now) / 0.7).clamp(0.0, 1.0);
    let fish_f = cfg.fish as f32 / FISH_MAX as f32;
    let speed_f = ((cfg.speed.ln() - 0.2f32.ln()) / (3.0f32.ln() - 0.2f32.ln())).clamp(0.0, 1.0);
    let normal = (0.55, 0.85, 0.95);
    let amber = (1.0, 0.72, 0.25);
    let dim = (0.85, 0.45, 0.40);
    let state = |at_min: bool, at_max: bool| if at_max { (amber, " MAX") } else if at_min { (dim, " MIN") } else { (normal, "") };
    let (fish_col, fish_tag) = state(cfg.fish == 0, cfg.fish >= FISH_MAX);
    let (speed_col, speed_tag) = state(cfg.speed <= 0.201, cfg.speed >= 2.999);
    let rgba = |c: (f32, f32, f32), a: f32| Color::srgba(c.0, c.1, c.2, a);
    for (role, bg, tc, node, text) in &mut q {
        match role {
            Role::HudPanel => if let Some(mut b) = bg { b.0 = Color::srgba(0.0, 0.03, 0.06, 0.55 * hud); },
            Role::HudLabel => if let Some(mut t) = tc { t.0 = Color::srgba(0.85, 0.93, 1.0, 0.9 * hud); },
            Role::BarBack => if let Some(mut b) = bg { b.0 = Color::srgba(1.0, 1.0, 1.0, 0.18 * hud); },
            Role::Msg => {
                if let Some(mut t) = tc { t.0 = Color::srgba(0.85, 0.93, 1.0, 0.85 * hud); }
                if let Some(mut t) = text { let m = if vtime.is_paused() { "Paused" } else { osd.msg.as_str() }; if t.0 != m { t.0 = m.to_string(); } }
            }
            Role::FishFill => {
                if let Some(mut b) = bg { b.0 = rgba(fish_col, 0.95 * hud); }
                if let Some(mut n) = node { n.width = Val::Percent(100.0 * fish_f); }
            }
            Role::SpeedFill => {
                if let Some(mut b) = bg { b.0 = rgba(speed_col, 0.95 * hud); }
                if let Some(mut n) = node { n.width = Val::Percent(100.0 * speed_f); }
            }
            Role::FishVal => {
                if let Some(mut t) = tc { t.0 = rgba(fish_col, 0.95 * hud); }
                if let Some(mut t) = text { t.0 = format!("{} / {}{}", cfg.fish, FISH_MAX, fish_tag); }
            }
            Role::SpeedVal => {
                if let Some(mut t) = tc { t.0 = rgba(speed_col, 0.95 * hud); }
                if let Some(mut t) = text { t.0 = format!("{:.2}x{}", cfg.speed, speed_tag); }
            }
            Role::HelpPanel => if let Some(mut b) = bg { b.0 = Color::srgba(0.0, 0.03, 0.06, 0.72 * help); },
            Role::HelpText => if let Some(mut t) = tc { t.0 = Color::srgba(0.9, 0.96, 1.0, 0.95 * help); },
        }
    }
}

// ---------------------------------------------------------------- crab, plankton, camera

#[derive(Component)]
struct Vignette;

#[derive(Component)]
struct SceneCam;

#[derive(Component)]
struct Crab { x0: f32, amp: f32, period: f32, roll: bool }

/// The hermit crab scuttles slowly back and forth along the sand, rocking as it walks.
fn crab_walk(time: Res<Time>, cfg: Res<Config>, mut q: Query<(&mut Transform, &Crab)>) {
    let t = time.elapsed_secs() * cfg.speed;
    for (mut tf, c) in &mut q {
        let ph = t * std::f32::consts::TAU / c.period;
        tf.translation.x = c.x0 + c.amp * ph.sin();
        if c.roll {
            let walking = ph.cos().abs();
            tf.rotation = Quat::from_rotation_z(0.04 * walking * (t * 8.0).sin());
            // faces the way it walks: it slows to a stop at each end and turns around while almost still
            tf.scale.x = (ph.cos() * 14.0).clamp(-1.0, 1.0);
        }
    }
}

#[derive(Component)]
struct Mote { id: u32 }

fn spawn_motes(commands: &mut Commands, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>, assets: &AssetServer, n: usize) {
    if n == 0 { return; }
    let mat = mats.add(StandardMaterial {
        base_color_texture: Some(assets.load("tex/mote.png")),
        base_color: Color::srgba(1.0, 1.0, 1.0, 0.7),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        fog_enabled: false,
        double_sided: true,
        cull_mode: None,
        ..default()
    });
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    for i in 0..n as u32 {
        commands.spawn((
            Mesh3d(quad.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::default(),
            bevy::camera::visibility::NoFrustumCulling,
            Mote { id: i },
        ));
    }
}

/// Specks of plankton drift on slow, overlapping currents.
fn animate_motes(time: Res<Time>, cfg: Res<Config>, mut q: Query<(&mut Transform, &Mote)>) {
    let t = time.elapsed_secs() * cfg.speed;
    for (mut tf, m) in &mut q {
        let k = m.id * 17 + 5;
        let (bx, by, bz) = (-1.1 + 2.2 * hash(k), FLOOR + 0.04 + 0.46 * hash(k + 1), -0.9 + 1.2 * hash(k + 2));
        let (a, b, c) = (hash(k + 3) * 6.28, hash(k + 4) * 6.28, hash(k + 5) * 6.28);
        let x = bx + 0.07 * (t * 0.13 + a).sin() + 0.03 * (t * 0.31 + b).sin();
        let y = by + 0.05 * (t * 0.11 + c).sin() + 0.02 * (t * 0.37 + a).sin();
        let z = bz + 0.05 * (t * 0.09 + b).sin();
        tf.translation = Vec3::new(x, y, z);
        tf.scale = Vec3::splat((0.0035 + 0.006 * hash(k + 6)) * (0.8 + 0.2 * (t * 1.7 + a).sin()));
    }
}

/// A very slow drift of the viewpoint, like a camera gently floating in front of the glass.
fn camera_drift(time: Res<Time>, cfg: Res<Config>, mut q: Query<&mut Transform, With<SceneCam>>) {
    let t = time.elapsed_secs() * cfg.speed;
    for mut tf in &mut q {
        let pos = Vec3::new(0.035 * (t * 0.05).sin(), -0.10 + 0.012 * (t * 0.037 + 1.0).sin(), 1.1 + 0.025 * (t * 0.031).sin());
        let look = Vec3::new(0.02 * (t * 0.043 + 2.0).sin(), -0.02 + 0.006 * (t * 0.05).sin(), 0.0);
        *tf = Transform::from_translation(pos).looking_at(look, Vec3::Y);
    }
}

/// The picture is shown on a grid whose corner vertices are darker: a soft vignette that costs nothing to draw
/// (a full-screen blended layer cost about 15 fps on the integrated GPU).
fn vignette_grid(nx: usize, ny: usize) -> Mesh {
    let mut pos = Vec::new();
    let mut uv = Vec::new();
    let mut col = Vec::new();
    for j in 0..=ny {
        for i in 0..=nx {
            let (u, v) = (i as f32 / nx as f32, j as f32 / ny as f32);
            let (x, y) = (u * 2.0 - 1.0, 1.0 - v * 2.0);
            pos.push([x, y, 0.0]);
            uv.push([u, v]);
            let r = ((x * 0.9).powi(2) + y * y).sqrt();
            let d = ((r - 0.55) / 0.85).clamp(0.0, 1.0).powf(1.6) * 0.5;
            col.push([1.0 - d, 1.0 - d * 0.92, 1.0 - d * 0.82, 1.0]);
        }
    }
    let w1 = (nx + 1) as u32;
    let mut idx = Vec::new();
    for j in 0..ny as u32 {
        for i in 0..nx as u32 {
            let a = j * w1 + i;
            idx.extend_from_slice(&[a, a + w1, a + 1, a + 1, a + w1, a + w1 + 1]);
        }
    }
    let n = pos.len();
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, col)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0f32, 0.0, 1.0]; n])
        .with_inserted_indices(Indices::U32(idx))
}

// ---------------------------------------------------------------- day and night

#[derive(Resource)]
struct DayCycle { offset: f32, d: f32, blend: f32 } // d: 1 in full daylight .. 0 at night; blend: 0 = cycle off .. 1 = on

#[derive(Resource)]
struct UpscaleMat(Handle<StandardMaterial>);

/// Position in the day, 0..1 (0 = midnight, 0.25 = dawn, 0.5 = noon, 0.75 = dusk).
fn day_phase(day: &DayCycle, elapsed: f32, day_min: f32) -> f32 {
    if day_min <= 0.0 { return 0.5; }
    (day.offset + elapsed / (day_min * 60.0)).rem_euclid(1.0)
}

fn mix3(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

/// Tints the whole picture: bright and neutral by day, warm at dawn and dusk, dim and blue at night;
/// the sun rays become faint moonbeams and the sand shimmer fades.
fn day_night(
    time: Res<Time>,
    cfg: Res<Config>,
    mut day: ResMut<DayCycle>,
    up: Res<UpscaleMat>,
    cm: Res<CausticMat>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    rays: Query<(&MeshMaterial3d<StandardMaterial>, &Ray)>,
) {
    // ease in and out when the cycle is switched on or off, so the light never jumps
    let target = if cfg.daynight { 1.0 } else { 0.0 };
    let dt = time.delta_secs().min(0.1);
    day.blend += (target - day.blend).clamp(-0.4 * dt, 0.4 * dt);
    let ph = day_phase(&day, time.elapsed_secs() * cfg.speed, cfg.day_min);
    let s = (std::f32::consts::TAU * (ph - 0.25)).sin(); // +1 noon, -1 midnight
    let cyc = { let x = ((s + 0.18) / 0.48).clamp(0.0, 1.0); x * x * (3.0 - 2.0 * x) };
    let d = 1.0 - day.blend * (1.0 - cyc);
    day.d = d;
    let warm = (1.0 - s.abs() / 0.32).clamp(0.0, 1.0) * 0.6 * day.blend;
    let tint = mix3(mix3((0.20, 0.30, 0.55), (1.0, 1.0, 1.0), d), (1.0, 0.76, 0.58), warm);
    if let Some(mut m) = mats.get_mut(&up.0) { m.base_color = Color::srgb(tint.0, tint.1, tint.2); }
    let c = mix3((0.03, 0.05, 0.09), (0.22, 0.34, 0.38), d);
    if let Some(mut m) = mats.get_mut(&cm.0) { m.base_color = Color::srgb(c.0, c.1, c.2); }
    let r = mix3((0.09, 0.13, 0.22), (0.16, 0.26, 0.28), d);
    for (h, ray) in &rays {
        if let Some(mut m) = mats.get_mut(&h.0) { m.base_color = Color::srgb(r.0 * ray.k, r.1 * ray.k, r.2 * ray.k); }
    }
}


// ---------------------------------------------------------------- feeding and the shark

#[derive(Resource)]
struct FeedReq(bool);

#[derive(Component)]
struct Flake { base_x: f32, phase: f32, sink: f32, spin: f32, size: f32, landed: f32, taken: bool }

/// E: a pinch of flakes at the surface. Every fish that has a flake ahead of it swims up to take it (fish never turn, so a fish
/// facing away from the spot simply skips this round).
fn feed_start(
    mut req: ResMut<FeedReq>,
    cfg: Res<Config>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    fish: Query<(&Transform, &Fish, &Visibility), Without<Shark>>,
    flakes: Query<(), With<Flake>>,
    mut cache: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
    mut seed: Local<u32>,
) {
    if !req.0 { return; }
    req.0 = false;
    if *seed == 0 { *seed = 0xC0FFEE; }
    if flakes.iter().count() > 70 { return; }
    let (mesh, mat) = cache.get_or_insert_with(|| (
        meshes.add(Rectangle::new(1.0, 1.0)),
        mats.add(StandardMaterial {
            base_color_texture: Some(assets.load("tex/flake.png")),
            base_color: Color::srgb(1.0, 1.0, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            fog_enabled: false,
            double_sided: true,
            cull_mode: None,
            ..default()
        }),
    )).clone();
    let top = 0.47;
    let spawn = |commands: &mut Commands, x: f32, y: f32, z: f32, seed: &mut u32| -> Entity {
        let size = 0.020 + 0.014 * rnd(seed);
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(x, y, z).with_scale(Vec3::splat(size)),
            Flake { base_x: x, phase: rnd(seed) * 6.28, sink: 0.040 + 0.025 * rnd(seed), spin: 0.4 + 1.2 * rnd(seed), size, landed: 0.0, taken: false },
        )).id()
    };
    // flakes fall in the depth lanes of the fish in the tank, so every fish has some within reach
    let lanes: Vec<f32> = fish.iter().filter(|(_, f, v)| !f.school && !f.low && !f.gone && f.idx < cfg.fish && **v != Visibility::Hidden).map(|(_, f, _)| f.home.z).collect();
    for i in 0..(lanes.len() * 2 + 4) {
        let z = if lanes.is_empty() { -0.30 + 0.44 * rnd(&mut seed) } else { lanes[i % lanes.len()] };
        let (x, y) = (-0.8 + 1.6 * rnd(&mut seed), top + 0.35 * rnd(&mut seed));
        spawn(&mut commands, x, y, z, &mut seed);
    }
}

/// Flakes sink slowly, wobbling a little; the ones nobody takes settle on the sand and dissolve.
fn animate_flakes(time: Res<Time>, cfg: Res<Config>, mut commands: Commands, mut q: Query<(Entity, &mut Transform, &mut Flake)>) {
    let dt = time.delta_secs().min(0.1) * cfg.speed;
    let t = time.elapsed_secs() * cfg.speed;
    for (e, mut tf, mut fl) in &mut q {
        if fl.landed > 0.0 {
            fl.landed += dt;
            let k = 1.0 - ((fl.landed - 8.0) / 3.0).clamp(0.0, 1.0);
            tf.scale = Vec3::splat(fl.size * k);
            if k <= 0.0 { commands.entity(e).despawn(); }
            continue;
        }
        tf.translation.y -= fl.sink * dt;
        tf.translation.x = fl.base_x + 0.02 * (t * 0.8 + fl.phase).sin();
        tf.rotation = Quat::from_rotation_z(fl.phase + t * fl.spin * 0.3);
        if tf.translation.y <= FLOOR + 0.02 {
            tf.translation.y = FLOOR + 0.015;
            fl.landed = 0.001;
        }
    }
}

#[derive(Resource)]
struct SharkClock { next: f32, summon: bool }

#[derive(Component)]
struct Shark { active: bool, prey: Option<Entity>, eats: u32 }

/// Now and then a shark crosses the tank in the depth lane of one of the fish, heads for it, and eats whatever it runs into.
/// Fish ahead of it, swimming the same way, bolt; an eaten fish is gone for a minute or two (a school member, until the school comes round).
fn shark_ai(
    time: Res<Time>,
    cfg: Res<Config>,
    mut clock: ResMut<SharkClock>,
    mut seed: Local<u32>,
    mut sq: Query<(&mut Transform, &mut Fish, &mut Visibility, &mut Shark)>,
    mut prey: Query<(Entity, &mut Transform, &mut Fish, &mut Visibility), Without<Shark>>,
) {
    if *seed == 0 { *seed = 0x5EED_1234; }
    let dt = time.delta_secs().min(0.1);
    let now = time.elapsed_secs();
    let Ok((mut tf, mut sf, mut sv, mut sh)) = sq.single_mut() else { return };
    let lim = 1.15 + sf.len * 0.5;
    if !sh.active {
        if clock.next == 0.0 { clock.next = now + 300.0 + 300.0 * rnd(&mut seed); }
        if !(clock.summon || (cfg.shark && now >= clock.next)) { return; }
        clock.summon = false;
        let cands: Vec<(Entity, f32, f32, f32, f32)> = prey.iter()
            .filter(|(_, t, f, v)| !f.school && !f.gone && f.wait <= 0.0 && f.idx < cfg.fish && **v != Visibility::Hidden && t.translation.x.abs() < 0.85)
            .map(|(e, t, f, _)| (e, t.translation.x, t.translation.y, t.translation.z, f.dir))
            .collect();
        if cands.is_empty() { clock.next = now + 60.0; return; }
        let k = (rnd(&mut seed) * cands.len() as f32) as usize % cands.len();
        let (e, _, py, pz, pdir) = cands[k];
        sf.dir = -pdir; // head-on to its target
        sf.yaw = if sf.dir > 0.0 { 0.0 } else { std::f32::consts::PI };
        tf.translation = Vec3::new(-sf.dir * lim, py.clamp(FLOOR + 0.12, 0.42), pz);
        tf.rotation = Quat::from_rotation_y(sf.yaw);
        sf.sp = 0.2;
        sf.vy = 0.0;
        sf.pitch = 0.0;
        *sh = Shark { active: true, prey: Some(e), eats: 0 };
        *sv = Visibility::Inherited;
        println!("shark arrives");
        return;
    }
    let cruise = 0.24 * cfg.speed;
    let mouth_x = tf.translation.x + sf.dir * sf.len * 0.5;
    let mouth_y = tf.translation.y - sf.height * 0.1;
    let mut want_sp = cruise;
    let mut vy_want = 0.0;
    if let Some(pe) = sh.prey {
        let mut keep = false;
        if let Ok((_, pt, pf, pv)) = prey.get(pe) {
            if *pv != Visibility::Hidden && !pf.gone && pf.wait <= 0.0 {
                let ahead = (pt.translation.x - mouth_x) * sf.dir;
                if ahead > -0.1 {
                    keep = true;
                    vy_want = ((pt.translation.y - mouth_y) * 1.2).clamp(-0.18, 0.18);
                    if ahead < 0.6 { want_sp = cruise * 1.6; }
                }
            }
        }
        if !keep { sh.prey = None; }
    }
    sf.sp += (want_sp - sf.sp) * (dt * 0.8).min(1.0);
    tf.translation.x += sf.dir * sf.sp * dt;
    sf.vy += (vy_want - sf.vy) * (dt * 1.5).min(1.0);
    tf.translation.y = (tf.translation.y + sf.vy * dt).clamp(FLOOR + 0.1, 0.45);
    sf.pitch += ((sf.vy / sf.sp.max(0.1)).atan().clamp(-0.3, 0.3) - sf.pitch) * (dt * 1.2).min(1.0);
    tf.rotation = Quat::from_euler(EulerRot::YXZ, sf.yaw, 0.0, sf.pitch * sf.dir);
    for (_, pt, mut pf, mut pv) in &mut prey {
        if *pv == Visibility::Hidden || pf.gone || (!pf.school && pf.wait > 0.0) { continue; }
        let dxm = (pt.translation.x - mouth_x) * sf.dir;
        let dy = (pt.translation.y - mouth_y).abs();
        let dz = (pt.translation.z - tf.translation.z).abs();
        if sh.eats < 2 && dxm > -0.15 && dxm < 0.09 && dy < 0.10 && dz < 0.22 {
            if pf.school { pf.gone = true; } else { pf.wait = 60.0 + 90.0 * rnd(&mut seed); pf.food = None; }
            *pv = Visibility::Hidden;
            sh.eats += 1;
            println!("shark ate a fish");
        } else if !pf.school && pf.dir == sf.dir && dxm > 0.0 && dxm < 0.8 && dz < 0.2 {
            pf.dart = pf.dart.max(1.2); // bolt away
            pf.hover = 0.0;
        }
    }
    if tf.translation.x * sf.dir > lim {
        sh.active = false;
        *sv = Visibility::Hidden;
        clock.next = now + 600.0 + 900.0 * rnd(&mut seed);
        println!("shark leaves");
    }
}

/// Dev aid: AQUARIUM_FEED_AT=secs presses the feed key then; AQUARIUM_SHARK_AT=secs calls the shark.
fn debug_triggers(time: Res<Time>, mut feed: ResMut<FeedReq>, mut clock: ResMut<SharkClock>, mut done: Local<(bool, bool)>) {
    let at = |k: &str| std::env::var(k).ok().and_then(|v| v.parse::<f32>().ok());
    let t = time.elapsed_secs();
    if let Some(a) = at("AQUARIUM_FEED_AT") { if !done.0 && t > a { done.0 = true; feed.0 = true; } }
    if let Some(a) = at("AQUARIUM_SHARK_AT") { if !done.1 && t > a { done.1 = true; clock.summon = true; } }
}
