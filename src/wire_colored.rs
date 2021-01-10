use bevy::prelude::*;

pub struct WireColoredPlugin;

impl Plugin for WireColoredPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system_to_stage(crate::RENDER_SETUP, update_wire_tint.system());
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WireColored {
    pub is_on: bool,
    pub on_color: Color,
    pub off_color: Color,
}

impl Default for WireColored {
    fn default() -> Self {
        Self {
            is_on: false,
            on_color: Color::rgb(1.0, 0.0, 0.0),
            off_color: Color::rgb(0.0, 0.0, 0.0),
        }
    }
}

fn update_wire_tint(
    mut materials: ResMut<Assets<ColorMaterial>>,
    query: Query<(&WireColored, &Handle<ColorMaterial>), Changed<WireColored>>,
) {
    for (wire_tint, material_handle) in query.iter() {
        if let Some(material) = materials.get_mut(material_handle) {
            if wire_tint.is_on {
                material.color = wire_tint.on_color;
            } else {
                material.color = wire_tint.off_color;
            }
        }
    }
}
