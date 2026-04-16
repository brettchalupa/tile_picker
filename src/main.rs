/*
 * Game dev tool for viewing a tilesheet and easily getting tile indexes. Made
 * by Brett Chalupa. Released into the public domain.
 */

use raylib::prelude::*;
use serde::Deserialize;

const MARGIN: f32 = 20.;
const CONFIG_FILE: &str = "tp.toml";

enum Scene {
    PickFile,
    ViewImage,
}

struct ViewImageState {
    /// width and height of each tile in pixels
    tile_size: i32,
    /// zoom for the texture
    zoom: f32,
    /// position to draw the active texture at, used for panning the image
    /// around the viewer
    pos: Vector2,
    /// Whether or not to show the grid and index on top of the tilesheet
    show_overlay: bool,
    background_color_idx: usize,
    /// When true, displayed and copied indexes are 1-based
    one_based_index: bool,
}

/// Temporarily displayed pop-up message to give feedback to the user after an
/// action is taken
struct Toast {
    /// time remaining to show the toast
    timer: f32,
    /// message to display
    message: String,
}

/// seconds to show the toast before it disappears
const TOAST_TIMER: f32 = 3.;
impl Toast {
    fn new(message: String) -> Self {
        Self {
            message,
            timer: TOAST_TIMER,
        }
    }
}

#[derive(Deserialize)]
struct Config {
    #[serde(default = "default_dir")]
    dir: String,
    #[serde(default)]
    one_based_index: bool,
}

fn default_dir() -> String {
    "assets".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dir: default_dir(),
            one_based_index: false,
        }
    }
}

fn load_config() -> Config {
    match std::fs::read_to_string(CONFIG_FILE) {
        Ok(contents) => match toml::from_str(&contents) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Warning: failed to parse {}: {}", CONFIG_FILE, e);
                Config::default()
            }
        },
        Err(_) => Config::default(),
    }
}

struct State {
    scene: Scene,
    active_texture: Option<Texture2D>,
    image_paths: Vec<String>,
    /// list view item idx being hovered
    list_focus: i32,
    /// scroll position of the list view
    list_scroll_index: i32,
    /// list view item idx that is selected
    list_active: i32,
    /// the previous list view item idx that is selected, used for tracking when `list_active` changes
    list_active_last: i32,
    view_state: Option<ViewImageState>,
    toast: Option<Toast>,
    one_based_index: bool,
}

fn main() {
    unsafe {
        raylib::ffi::SetConfigFlags(raylib::ffi::ConfigFlags::FLAG_WINDOW_HIGHDPI as u32);
    }
    let (mut rl, thread) = raylib::init()
        .size(1280, 720)
        .resizable()
        .title("tile_picker")
        .build();

    rl.set_target_fps(60);

    let config = load_config();
    let dir = std::env::args().nth(1).unwrap_or(config.dir);
    let image_paths = rl
        .load_directory_files_ex(&dir, ".png".to_string(), false)
        .paths()
        .iter()
        .map(|f| f.to_string())
        .collect();

    let mut state = State {
        scene: Scene::PickFile,
        active_texture: None,
        image_paths,
        list_focus: 0,
        list_scroll_index: 0,
        list_active: 0,
        list_active_last: 0,
        view_state: None,
        toast: None,
        one_based_index: config.one_based_index,
    };

    load_active_into_texture(
        &mut rl,
        &thread,
        &state.image_paths,
        state.list_active,
        &mut state.active_texture,
    );
    while !rl.window_should_close() {
        if let Some(toast) = &mut state.toast {
            toast.timer -= rl.get_frame_time();

            if toast.timer <= 0. {
                state.toast = None;
            }
        }

        match state.scene {
            Scene::PickFile => {
                update_pick_file(&mut rl, &thread, &mut state);
            }
            Scene::ViewImage => {
                update_view_image(&mut rl, &thread, &mut state);
            }
        }
    }
}

const PAN_SPEED: f32 = 20.;
const ZOOM_STEP: f32 = 0.5;
const BACKGROUND_COLORS: [Color; 3] = [Color::GRAY, Color::BLACK, Color::WHITE];

fn update_view_image(rl: &mut RaylibHandle, thread: &RaylibThread, state: &mut State) {
    let Some(view_state) = &mut state.view_state else {
        return;
    };
    let Some(texture) = &state.active_texture else {
        return;
    };

    let mouse_pos = rl.get_mouse_position();
    let tiles_wide = texture.width / view_state.tile_size;
    let tiles_high = texture.height / view_state.tile_size;
    let tiles_count = tiles_wide * tiles_high;
    let screen_width = rl.get_screen_width() as f32;
    let screen_height = rl.get_screen_height() as f32;

    if rl.is_key_down(KeyboardKey::KEY_D) {
        view_state.pos.x -= PAN_SPEED;
    }
    if rl.is_key_down(KeyboardKey::KEY_A) {
        view_state.pos.x += PAN_SPEED;
    }
    if rl.is_key_down(KeyboardKey::KEY_S) {
        view_state.pos.y -= PAN_SPEED;
    }
    if rl.is_key_down(KeyboardKey::KEY_W) {
        view_state.pos.y += PAN_SPEED;
    }
    if rl.is_key_pressed(KeyboardKey::KEY_R) {
        view_state.show_overlay = !view_state.show_overlay;
    }
    if rl.is_key_pressed(KeyboardKey::KEY_Q) {
        view_state.zoom -= ZOOM_STEP;
    }
    if rl.is_key_pressed(KeyboardKey::KEY_E) {
        view_state.zoom += ZOOM_STEP;
    }
    if rl.is_key_pressed(KeyboardKey::KEY_B) {
        view_state.background_color_idx += 1;
        if view_state.background_color_idx >= BACKGROUND_COLORS.len() {
            view_state.background_color_idx = 0;
        }
    }
    if rl.is_key_pressed(KeyboardKey::KEY_ONE) {
        view_state.one_based_index = !view_state.one_based_index;
        let mode = if view_state.one_based_index {
            "1-indexed (Lua)"
        } else {
            "0-indexed"
        };
        state.toast = Some(Toast::new(format!("Switched to {}", mode)));
    }
    view_state.zoom = view_state.zoom.clamp(0.5, 20.);

    if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
        && mouse_pos.x >= view_state.pos.x
        && mouse_pos.x <= view_state.pos.x + texture.width as f32 * view_state.zoom
        && mouse_pos.y >= view_state.pos.y
        && mouse_pos.y <= view_state.pos.y + texture.height as f32 * view_state.zoom
    {
        let tile_x = ((mouse_pos.x - view_state.pos.x)
            / (view_state.tile_size as f32 * view_state.zoom))
            .floor() as i32;
        let tile_y = ((mouse_pos.y - view_state.pos.y)
            / (view_state.tile_size as f32 * view_state.zoom))
            .floor() as i32;
        let idx = tiles_wide * tile_y + tile_x + if view_state.one_based_index { 1 } else { 0 };
        rl.set_clipboard_text(&idx.to_string())
            .expect("Writing to clipboard failed");
        let msg = format!("Copied to clipboard: {}", idx);
        println!("{}", msg);
        state.toast = Some(Toast::new(msg))
    }

    let mut d = rl.begin_drawing(thread);
    d.clear_background(BACKGROUND_COLORS[view_state.background_color_idx]);

    d.draw_texture_ex(texture, view_state.pos, 0., view_state.zoom, Color::WHITE);

    if view_state.show_overlay {
        let overlay_color = Color::alpha(&Color::DARKCYAN, 0.8);

        let index_offset = if view_state.one_based_index { 1 } else { 0 };
        // draw indexes
        for idx in 0..tiles_count {
            let padding = 4.;
            let x = view_state.pos.x
                + ((idx % tiles_wide) as f32 * (view_state.tile_size as f32 * view_state.zoom))
                + padding;
            let y = view_state.pos.y
                + ((idx / tiles_wide) as f32 * (view_state.tile_size as f32 * view_state.zoom))
                + padding;
            d.draw_text(
                &(idx + index_offset).to_string(),
                x as i32,
                y as i32,
                (12. * view_state.zoom / 3.) as i32,
                overlay_color,
            );
        }

        // draw lines
        for row in 0..=tiles_high {
            d.draw_line_ex(
                Vector2::new(
                    view_state.pos.x,
                    view_state.pos.y + (row as f32 * view_state.tile_size as f32 * view_state.zoom),
                ),
                Vector2::new(
                    view_state.pos.x + (texture.width as f32 * view_state.zoom),
                    view_state.pos.y + (row as f32 * view_state.tile_size as f32 * view_state.zoom),
                ),
                2. * view_state.zoom / 3.,
                overlay_color,
            );
        }
        for col in 0..=tiles_wide {
            d.draw_line_ex(
                Vector2::new(
                    view_state.pos.x + (col as f32 * view_state.tile_size as f32 * view_state.zoom),
                    view_state.pos.y,
                ),
                Vector2::new(
                    view_state.pos.x + (col as f32 * view_state.tile_size as f32 * view_state.zoom),
                    view_state.pos.y + (texture.height as f32 * view_state.zoom),
                ),
                2. * view_state.zoom / 4.,
                overlay_color,
            );
        }
    }

    d.gui_panel(
        Rectangle::new(MARGIN - 8., MARGIN - 8., 160., 120.),
        state.image_paths.get(state.list_active as usize).unwrap(),
    );
    d.draw_text(
        &format!("{}x{}px", texture.width, texture.height),
        MARGIN as i32,
        (MARGIN * 2.2) as i32,
        16,
        Color::BLACK,
    );
    d.draw_text(
        &format!("Tiles: {}", tiles_count),
        MARGIN as i32,
        (MARGIN * 3.2) as i32,
        16,
        Color::BLACK,
    );
    d.draw_text(
        &format!("Tile size: {}px", view_state.tile_size),
        MARGIN as i32,
        (MARGIN * 4.2) as i32,
        16,
        Color::BLACK,
    );
    d.draw_text(
        &format!("Zoom: {}x", view_state.zoom),
        MARGIN as i32,
        (MARGIN * 5.2) as i32,
        16,
        Color::BLACK,
    );

    // Close button
    let btn_w = 32.;
    if d.gui_button(
        Rectangle::new(screen_width - btn_w - MARGIN, MARGIN, btn_w, 32.),
        "x",
    ) {
        state.scene = Scene::PickFile;
    }

    if let Some(toast) = &mut state.toast {
        let panel_w = 200.;
        let panel_h = 64.;
        let panel_rect = Rectangle::new(
            screen_width - panel_w - MARGIN,
            screen_height - panel_h - MARGIN,
            panel_w,
            panel_h,
        );
        d.gui_panel(panel_rect, "TOAST");
        d.draw_text(
            &toast.message,
            (panel_rect.x + 8.) as i32,
            (panel_rect.y + 38.) as i32,
            12,
            Color::BLACK,
        );
    }
}

fn update_pick_file(rl: &mut RaylibHandle, thread: &RaylibThread, state: &mut State) {
    let screen_width = rl.get_screen_width() as f32;

    if rl.is_key_pressed(KeyboardKey::KEY_W) || rl.is_key_pressed(KeyboardKey::KEY_UP) {
        state.list_active -= 1;
    }
    if rl.is_key_pressed(KeyboardKey::KEY_S) || rl.is_key_pressed(KeyboardKey::KEY_DOWN) {
        state.list_active += 1;
    }
    let image_count = state.image_paths.len() as i32;
    if state.list_active < 0 {
        state.list_active = image_count - 1;
    }
    if state.list_active >= image_count {
        state.list_active = 0
    }
    if rl.is_key_pressed(KeyboardKey::KEY_SPACE) || rl.is_key_pressed(KeyboardKey::KEY_ENTER) {
        switch_scene(state, Scene::ViewImage);
    }

    if state.list_active_last != state.list_active {
        load_active_into_texture(
            rl,
            thread,
            &state.image_paths,
            state.list_active,
            &mut state.active_texture,
        );
        state.list_active_last = state.list_active;
    }

    let mut d = rl.begin_drawing(thread);
    d.clear_background(Color::WHITE);

    d.gui_list_view_ex(
        Rectangle::new(20., 20., 500., 560.),
        state.image_paths.iter(),
        &mut state.list_scroll_index,
        &mut state.list_active,
        &mut state.list_focus,
    );

    if let Some(texture) = &state.active_texture {
        let scale = if texture.width < 400 { 2. } else { 1. };
        d.draw_texture_ex(texture, Vector2::new(600., MARGIN), 0., scale, Color::WHITE)
    }

    if state.list_active >= 0 && state.active_texture.is_some() {
        let btn_w = 64.;
        if d.gui_button(
            Rectangle::new(screen_width - btn_w - MARGIN, MARGIN, btn_w, 32.),
            "View",
        ) {
            switch_scene(state, Scene::ViewImage);
        }
    }
}

fn switch_scene(state: &mut State, next_scene: Scene) {
    match next_scene {
        Scene::PickFile => {
            state.view_state = None;
        }
        Scene::ViewImage => {
            state.view_state = Some(ViewImageState {
                zoom: 2.,
                pos: Vector2::new(240., 60.),
                show_overlay: true,
                tile_size: determine_tile_size(
                    state.image_paths.get(state.list_active as usize).unwrap(),
                ),
                background_color_idx: 0,
                one_based_index: state.one_based_index,
            });
        }
    }
    state.scene = next_scene;
}

/// Parses out the tile size from the Playdate image table format:
/// `name-table-W-H.png` where W is width and H is height.
/// Assumes square tiles and uniform tile size. Defaults to 16.
fn determine_tile_size(file_name: &str) -> i32 {
    let stem = file_name.trim_end_matches(".png");
    if let Some(table_pos) = stem.find("-table-") {
        let dims = &stem[table_pos + 7..]; // skip "-table-"
        if let Some((w, _h)) = dims.split_once('-')
            && let Ok(size) = w.parse::<i32>()
        {
            return size;
        }
    }
    16 // default
}

/// Reads the image_path at `idx` from disk and loads into the `texture`, used
/// for previewing in the file picker and then the view image scene
fn load_active_into_texture(
    rl: &mut RaylibHandle,
    thread: &RaylibThread,
    image_paths: &[String],
    idx: i32,
    texture: &mut Option<Texture2D>,
) {
    let len = image_paths.len() as i32;

    if idx < 0 || len <= 0 || idx > len - 1 {
        *texture = None;
        return;
    }

    let path = image_paths.get(idx as usize).expect("Missing image path");
    *texture = Some(
        rl.load_texture(thread, path)
            .expect("Couldn't load texture"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_config(input: &str) -> Config {
        toml::from_str(input).unwrap()
    }

    #[test]
    fn test_config_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config.dir, "assets");
        assert!(!config.one_based_index);
    }

    #[test]
    fn test_config_full() {
        let config = parse_config(
            r#"
            dir = "sprites"
            one_based_index = true
            "#,
        );
        assert_eq!(config.dir, "sprites");
        assert!(config.one_based_index);
    }

    #[test]
    fn test_config_partial_dir_only() {
        let config = parse_config(r#"dir = "gfx""#);
        assert_eq!(config.dir, "gfx");
        assert!(!config.one_based_index);
    }

    #[test]
    fn test_config_partial_index_only() {
        let config = parse_config("one_based_index = true");
        assert_eq!(config.dir, "assets");
        assert!(config.one_based_index);
    }

    #[test]
    fn test_config_invalid_toml_falls_back_to_default() {
        let config: Result<Config, _> = toml::from_str("not valid toml {{{}}}");
        assert!(config.is_err());
        let fallback = Config::default();
        assert_eq!(fallback.dir, "assets");
        assert!(!fallback.one_based_index);
    }

    #[test]
    fn test_determine_tile_size() {
        assert_eq!(determine_tile_size("tiles-table-32-32.png"), 32);
        assert_eq!(determine_tile_size("ships-table-64-64.png"), 64);
        assert_eq!(determine_tile_size("path/to/sprites-table-16-16.png"), 16);
        assert_eq!(determine_tile_size("file_name.png"), 16); // default case
    }
}
