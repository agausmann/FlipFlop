use bevy::prelude::*;

pub struct ColoredPlugin;

impl Plugin for ColoredPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_to_stage(crate::RENDER_SETUP, colored_update.system());
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Colored {
    pub color: Color,
}

fn colored_update(
    query: Query<(&Colored, &Handle<ColorMaterial>), Changed<Colored>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (colored, handle) in query.iter() {
        if let Some(material) = materials.get_mut(handle) {
            material.color = colored.color;
        }
    }
}
