use bevy::prelude::*;

mod stepping;

const SCOREBOARD_FONT_SIZE: f32 = 33.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

const BACKGROUND_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const TEXT_COLOR: Color = Color::srgb(0.5, 0.5, 1.0);
const GREEN_TEXT: Color = Color::srgb(0.5, 1.0, 0.5);
const RED_TEXT: Color = Color::srgb(1.0, 0.5, 0.5);

const SCORE_COLOR: Color = Color::srgb(1.0, 0.5, 0.5);

const GEM_SIZE: f32 = 25.;
const PLAYER_SIZE: f32 = 100.;
const MAX_HEALTH: i32 = 3;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(
            stepping::SteppingPlugin::default()
                .add_schedule(Update)
                .add_schedule(FixedUpdate)
                .at(Val::Percent(35.0), Val::Percent(50.0)),
        )
        .insert_resource(Score(0))
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_event::<CollisionEvent>()
        .add_systems(Startup, setup)
        .insert_state(GameState::Playing)
        // Add our gameplay simulation systems to the fixed timestep schedule
        // which runs at 64 Hz by default
        .add_systems(
            FixedUpdate,
            (move_player, follow_player, collect_gems)
                // `chain`ing systems together runs them in order
                .chain()
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            (update_scoreboard, update_health_ui).run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, check_player_death)
        .add_systems(OnEnter(GameState::GameOver), show_game_over)
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Health {
    current: i32,
    max: i32,
}

#[derive(Component)]
struct Gem;

#[derive(Resource, Deref)]
struct CollisionSound(Handle<AudioSource>);

#[derive(Component)]
struct Collider;

#[derive(Event, Default)]
struct CollisionEvent;

#[derive(Resource, Deref, DerefMut)]
struct Score(usize);

// UIs
#[derive(Component)]
struct ScoreboardUi;

#[derive(Component)]
struct HealthUi;

#[derive(Component)]
struct GameOverUi;

// Game state
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum GameState {
    #[default]
    Playing,
    GameOver,
}

fn move_player(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_transform: Single<&mut Transform, With<Player>>,
    time: Res<Time>,
) {
    let mut vertical = 0.0;

    if keyboard_input.pressed(KeyCode::ArrowUp) {
        vertical += 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) {
        vertical -= 1.0;
    }

    let horizontal_speed = 300.0;
    let vertical_speed = 300.0;

    let movement = Vec3::new(
        horizontal_speed * time.delta_secs(), // constant scroll right
        vertical * vertical_speed * time.delta_secs(), // up/down input
        0.0,
    );

    player_transform.translation += movement;
}

fn follow_player(
    player_transform: Query<&Transform, With<Player>>,
    mut camera_transform: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let player = player_transform.single();
    let mut camera = camera_transform.single_mut();
    camera.translation.x = player.translation.x + 200.0; // Look ahead a bit
}

fn collect_gems(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut player_query: Query<(&Transform, &mut Health), With<Player>>,
    gem_query: Query<(Entity, &Transform), With<Gem>>,
    sound: Res<CollisionSound>,
) {
    let (player_transform, mut health) = player_query.single_mut();
    let player_pos = player_transform.translation.truncate();

    for (gem_entity, transform) in &gem_query {
        if player_pos.distance(transform.translation.truncate()) < 30.0 {
            // Remove gem entity
            commands.entity(gem_entity).despawn();

            // Update score
            **score += 1;

            // Simulate health loss for demo
            health.current = (health.current - 1).max(0);

            // Play sound effect
            commands.spawn((AudioPlayer(sound.clone()), PlaybackSettings::DESPAWN));
        }
    }
}

// Add the game's entities to our world
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn Camera
    commands.spawn(Camera2d);

    // Spawn Player
    commands.spawn((
        Sprite {
            image: asset_server.load("sprites/rug.png"),
            custom_size: Some(Vec2::new(PLAYER_SIZE, PLAYER_SIZE)),
            ..default()
        },
        Player,
        Health {
            current: MAX_HEALTH,
            max: MAX_HEALTH,
        },
    ));

    // Spawn Gems
    for i in 0..100 {
        let x = i as f32 * 300.0 + 600.0; // Spread out along the scroll
        let y = rand::random::<f32>() * 400.0 - 200.0;

        commands.spawn((
            Sprite {
                image: asset_server.load("sprites/gem.png"),
                custom_size: Some(Vec2::new(GEM_SIZE, GEM_SIZE)),
                ..default()
            },
            Transform {
                translation: Vec3::new(x, y, 0.0),
                // scale: Vec3::splat(20.0),
                ..default()
            },
            Gem,
            Collider,
        ));
    }

    // Add Sound (gets played by the gem collection function)
    let ball_collision_sound = asset_server.load("sounds/gem_collection.ogg");
    commands.insert_resource(CollisionSound(ball_collision_sound));

    // Game Over UI
    commands
        .spawn((
            // FIXME: This doesn't center! How do I make it work?
            Node {
                position_type: PositionType::Absolute,
                // display: Display::Flex,
                // width: Val::Percent(100.0),
                // height: Val::Percent(100.0),
                // justify_content: JustifyContent::Center,
                // align_items: AlignItems::Center,
                top: SCOREBOARD_TEXT_PADDING * 20.0,
                left: SCOREBOARD_TEXT_PADDING * 20.0,
                ..default()
            },
            Text::new(""), // Empty string -- invisible but we will append to it when the game is over
            TextFont {
                font_size: SCOREBOARD_FONT_SIZE,
                ..default()
            },
            TextColor(TEXT_COLOR),
            GameOverUi,
        ))
        .with_child((
            TextSpan::default(),
            TextFont {
                font_size: SCOREBOARD_FONT_SIZE * 4.0,
                ..default()
            },
            TextColor(RED_TEXT),
        ));

    // Scoreboard UI
    commands
        .spawn((
            Text::new("Score: "),
            TextFont {
                font_size: SCOREBOARD_FONT_SIZE,
                ..default()
            },
            TextColor(TEXT_COLOR),
            ScoreboardUi,
            Node {
                position_type: PositionType::Absolute,
                top: SCOREBOARD_TEXT_PADDING,
                left: SCOREBOARD_TEXT_PADDING,
                ..default()
            },
        ))
        .with_child((
            TextSpan::default(),
            TextFont {
                font_size: SCOREBOARD_FONT_SIZE,
                ..default()
            },
            TextColor(SCORE_COLOR),
        ));

    // Health UI
    commands
        .spawn((
            Text::new("Health: "),
            TextFont {
                font_size: SCOREBOARD_FONT_SIZE,
                ..default()
            },
            TextColor(TEXT_COLOR),
            HealthUi,
            Node {
                position_type: PositionType::Absolute,
                top: SCOREBOARD_TEXT_PADDING * 10.0,
                left: SCOREBOARD_TEXT_PADDING,
                ..default()
            },
        ))
        .with_child((
            TextSpan::default(),
            TextFont {
                font_size: SCOREBOARD_FONT_SIZE,
                ..default()
            },
            TextColor(GREEN_TEXT),
        ));
}

fn check_player_death(
    player: Query<&Health, With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let health = player.single();
    if health.current <= 0 {
        // println!("Game Over!");
        next_state.set(GameState::GameOver);
    }
}

fn show_game_over(
    state: Res<State<GameState>>,
    game_over_root: Single<Entity, (With<GameOverUi>, With<Text>)>,
    mut writer: TextUiWriter,
) {
    let message = match state.get() {
        GameState::GameOver => "YOU DIED",
        _ => "", // Clear the message if not dead
    };

    *writer.text(*game_over_root, 1) = message.to_string();
}

fn update_health_ui(
    player: Query<&Health, With<Player>>,
    health_root: Single<Entity, (With<HealthUi>, With<Text>)>,
    mut writer: TextUiWriter,
) {
    let health = player.single();
    *writer.text(*health_root, 1) = format!("{}/{}", health.current, health.max);
}

fn update_scoreboard(
    score: Res<Score>,
    score_root: Single<Entity, (With<ScoreboardUi>, With<Text>)>,
    mut writer: TextUiWriter,
) {
    *writer.text(*score_root, 1) = score.to_string();
}
