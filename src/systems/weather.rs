use bevy::prelude::*;

use crate::components::WeatherMaterial;

pub fn update_weather_material(time: Res<Time>, mut materials: ResMut<Assets<WeatherMaterial>>) {
    let elapsed = time.elapsed_secs();

    for (_, material) in materials.iter_mut() {
        material.weather_data.x = elapsed;
    }
}