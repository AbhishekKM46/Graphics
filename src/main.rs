use bevy::prelude::*;
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_rapier3d::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::srgb(0.5, 0.8, 0.9))) // Sky Blue
        .add_plugins(PanOrbitCameraPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugins(RapierDebugRenderPlugin::default()) // Uncomment for debugging
        .add_systems(Startup, setup)
        .add_systems(Update, (animate_light, animate_droplet, reset_droplet, splash_on_impact))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 1.5, 5.0)),
            ..default()
        },
        PanOrbitCamera::default(),
    ));

    // Main Light (Sun-like)
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, -0.5, 0.0)),
        ..default()
    });
    
    // Ambient Light (Soft fill)
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 500.0,
    });

    // Floor (Checkerboard pattern would be nice, but simple light gray for now to show shadows)
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(20.0, 20.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.8, 0.8, 0.8),
            perceptual_roughness: 0.5,
            reflectance: 0.2,
            ..default()
        }),
        ..default()
    });

    // Sky Dome (Provides environment for reflections/refractions)
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Sphere::new(50.0))),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.8, 0.9), // Sky Blue
                unlit: true,
                cull_mode: None,
                ..default()
            }),
            ..default()
        },
        bevy::pbr::NotShadowCaster, // IMPORTANT: Don't block the sun!
        bevy::pbr::NotShadowReceiver,
    ));

    // Floor with Checkerboard Pattern
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(create_checkerboard_image())),
        perceptual_roughness: 0.8,
        reflectance: 0.2,
        ..default()
    });

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(20.0, 20.0)),
            material: debug_material,
            ..default()
        },
        RigidBody::Fixed,
        Collider::cuboid(10.0, 0.01, 10.0), // Half-extents
    ));

    // Water Droplet
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(Sphere::new(0.5))),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 0.01,
                metallic: 0.0,
                reflectance: 0.02,
                ior: 1.33,
                alpha_mode: AlphaMode::Opaque,
                specular_transmission: 1.0,
                thickness: 0.9,
                attenuation_color: Color::WHITE,
                attenuation_distance: 100.0,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 5.0, 0.0), // Start higher to fall
            ..default()
        },
        Droplet,
        RigidBody::Dynamic,
        Collider::ball(0.5),
        Restitution::coefficient(0.05), // Low bounce, mostly splash
        Damping { linear_damping: 0.5, angular_damping: 0.5 },
        Velocity::zero(), // Explicitly add Velocity so we can query it later
        ActiveEvents::COLLISION_EVENTS, // Listen for collisions
    ));
}

fn create_checkerboard_image() -> Image {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    const TEXTURE_SIZE: usize = 512;
    let mut palette: [u8; TEXTURE_SIZE * TEXTURE_SIZE * 4] = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];

    for y in 0..TEXTURE_SIZE {
        for x in 0..TEXTURE_SIZE {
            let i = (y * TEXTURE_SIZE + x) * 4;
            // 8x8 checkerboard
            let is_white = ((x / 64) + (y / 64)) % 2 == 0;
            let color = if is_white { 255 } else { 150 }; // White and Grey

            palette[i] = color;
            palette[i + 1] = color;
            palette[i + 2] = color;
            palette[i + 3] = 255;
        }
    }

    Image::new(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        palette.to_vec(),
        TextureFormat::Rgba8UnormSrgb,
        bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD,
    )
}

#[derive(Component)]
struct Droplet;

#[derive(Component)]
struct RotateLight;

fn animate_light(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<RotateLight>>,
) {
    for mut transform in query.iter_mut() {
        transform.translation = Vec3::new(
            4.0 * time.elapsed_seconds().cos(),
            8.0,
            4.0 * time.elapsed_seconds().sin(),
        );
    }
}

fn animate_droplet(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<Droplet>>,
) {
    for mut transform in query.iter_mut() {
        let t = time.elapsed_seconds();
        
        // Ripple effect (scaling on axes to simulate surface tension/ripples)
        // A more complex vertex shader would be better for surface ripples, 
        // but scaling works for a "wobbly droplet" feel.
        // To do actual surface ripples we'd need a custom shader or modifying mesh vertices every frame (expensive).
        // Let's stick to a more complex wobble that feels like ripples passing through.
        
        let wobble_x = (t * 5.0).sin() * 0.02;
        let wobble_y = (t * 4.3).cos() * 0.02;
        let wobble_z = (t * 3.5).sin() * 0.02;

        // Only wobble if not splashed (scale is close to 1.0)
        if transform.scale.y > 0.5 {
            transform.scale = Vec3::new(
                1.0 + wobble_x,
                1.0 + wobble_y, 
                1.0 + wobble_z,
            );
        }
    }
}

#[derive(Component)]
struct SplashParticle;

#[derive(Component)]
struct HasSplashed;

fn splash_on_impact(
    mut commands: Commands,
    mut collision_events: EventReader<CollisionEvent>,
    mut droplet_query: Query<(Entity, &mut Transform, &mut Visibility), (With<Droplet>, Without<HasSplashed>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for event in collision_events.read() {
        if let CollisionEvent::Started(e1, e2, _) = event {
            if let Ok((droplet_entity, mut transform, _)) = droplet_query.get_single_mut() {
                // Check if the droplet was involved in this collision
                if *e1 == droplet_entity || *e2 == droplet_entity {
                    // Only splash if we are falling fast enough (to avoid splashing while resting)
                    // Ideally check velocity, but for now let's just do it once per drop.
                    if transform.translation.y < 1.0 { 
                         // Flatten the droplet
                        transform.scale = Vec3::new(2.0, 0.1, 2.0);
                        
                        // Mark as splashed so it doesn't splash again
                        commands.entity(droplet_entity).insert(HasSplashed);

                        // Spawn Particles
                        let particle_material = materials.add(StandardMaterial {
                            base_color: Color::WHITE,
                            perceptual_roughness: 0.01,
                            metallic: 0.0,
                            reflectance: 0.02,
                            ior: 1.33,
                            alpha_mode: AlphaMode::Opaque,
                            specular_transmission: 1.0,
                            thickness: 0.1,
                            ..default()
                        });

                        let particle_mesh = meshes.add(Mesh::from(Sphere::new(0.1)));

                        for _ in 0..20 {
                            let mut rng = rand::thread_rng();
                            use rand::Rng;
                            let x_vel = rng.gen_range(-2.0..2.0);
                            let z_vel = rng.gen_range(-2.0..2.0);
                            let y_vel = rng.gen_range(2.0..5.0);

                            commands.spawn((
                                PbrBundle {
                                    mesh: particle_mesh.clone(),
                                    material: particle_material.clone(),
                                    transform: Transform::from_translation(transform.translation),
                                    ..default()
                                },
                                RigidBody::Dynamic,
                                Collider::ball(0.1),
                                Velocity {
                                    linvel: Vec3::new(x_vel, y_vel, z_vel),
                                    angvel: Vec3::ZERO,
                                },
                                SplashParticle,
                            ));
                        }
                    }
                }
            }
        }
    }
}

fn reset_droplet(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Velocity), With<Droplet>>,
    particle_query: Query<Entity, With<SplashParticle>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        // Reset Droplet
        for (entity, mut transform, mut velocity) in query.iter_mut() {
            transform.translation = Vec3::new(0.0, 5.0, 0.0);
            transform.scale = Vec3::ONE; // Un-flatten
            velocity.linvel = Vec3::ZERO;
            velocity.angvel = Vec3::ZERO;
            
            commands.entity(entity).remove::<HasSplashed>();
        }

        // Remove old particles
        for entity in particle_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}
