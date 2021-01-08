use bevy::prelude::*;

pub struct ColoredPlugin;

impl Plugin for ColoredPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(colored_update.system());
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Colored {
    pub color: Color,
}

fn colored_update(
    query: Query<(&Colored, &Handle<ColorMaterial>), Changed<Color>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (colored, handle) in query.iter() {
        if let Some(material) = materials.get_mut(handle) {
            material.color = colored.color;
        }
    }
}
