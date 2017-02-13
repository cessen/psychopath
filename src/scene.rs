use assembly::Assembly;
use camera::Camera;
use world::World;



#[derive(Debug)]
pub struct Scene {
    pub name: Option<String>,
    pub camera: Camera,
    pub world: World,
    pub root: Assembly,
}
