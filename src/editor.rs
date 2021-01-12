use crate::cursor::Cursor;
use crate::direction::Direction;
use crate::pin::Pin;
use crate::tile::Tile;
use crate::wire::Wire;
use bevy::input::mouse::{MouseButton, MouseButtonInput};
use bevy::input::ElementState;
use bevy::prelude::*;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(update_editor.system());
    }
}

#[derive(Default)]
struct Editor {
    editing: Option<Editing>,
}

enum Editing {
    Wire {
        wire: Entity,
        start: Entity,
        end: Entity,
    },
}

fn update_editor(
    commands: &mut Commands,
    cursor: Res<Cursor>,
    events: Res<Events<MouseButtonInput>>,
    mut reader: Local<EventReader<MouseButtonInput>>,
    mut editor: Local<Editor>,
    mut wires: Query<&mut Wire>,
    mut pins: Query<&mut Pin>,
) {
    match &editor.editing {
        Some(Editing::Wire { wire, end, .. }) => {
            let mut wire = wires.get_mut(*wire).expect("missing wire");
            let mut end = pins.get_mut(*end).expect("missing end pin");
            let cursor_distance = Vec2::from(cursor.tile) - Vec2::from(wire.start);

            // Set the wire's direction to the one nearest the cursor.
            wire.direction = Direction::nearest(cursor_distance);
            // Preserve the axis in the selected direction, and zero out the other one.
            let projected_distance = cursor_distance * wire.direction.vector().abs();

            wire.length = projected_distance.length() as i32;
            end.position = Tile::from(Vec2::from(wire.start) + projected_distance);
        }
        None => {}
    }

    for ev in reader.iter(&events) {
        match (ev.button, ev.state) {
            (MouseButton::Left, ElementState::Pressed) => {
                commands.spawn((Wire {
                    start: cursor.tile,
                    length: 0,
                    ..Default::default()
                },));
                let wire = commands.current_entity().unwrap();

                commands.spawn((Pin {
                    position: cursor.tile,
                    ..Default::default()
                },));
                let start = commands.current_entity().unwrap();

                commands.spawn((Pin {
                    position: cursor.tile,
                    ..Default::default()
                },));
                let end = commands.current_entity().unwrap();

                editor.editing = Some(Editing::Wire { wire, start, end });
            }
            (MouseButton::Left, ElementState::Released) => {
                editor.editing = None;
            }
            _ => {}
        }
    }
}
