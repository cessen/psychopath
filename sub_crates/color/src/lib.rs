#[allow(non_camel_case_types)]
#[derive(Copy, Clone)]
pub enum Space {
    XYZ,
    ACES_AP0,
    ACES_AP1,
    Rec709,
    Rec2020,
}

// Generated conversion functions between XYZ and various RGB colorspaces
include!(concat!(env!("OUT_DIR"), "/rec709_inc.rs"));
include!(concat!(env!("OUT_DIR"), "/rec2020_inc.rs"));
include!(concat!(env!("OUT_DIR"), "/aces_ap0_inc.rs"));
include!(concat!(env!("OUT_DIR"), "/aces_ap1_inc.rs"));
