use std::time::Duration;


pub struct Player {
    name: String,
    class: Class,
    statics: Statics,
    // graphics
}

pub struct Class {
    name: String,
    color: Color,
    abilities: Vec<Ability>,
}

pub struct Color {
    r: u8,
    g: u8,
    b: u8,
}

pub struct Ability {
    cooldown: Duration,
    expr: fn(&Statics) -> i32,
    // graphics
    // icon
    // description
}

pub struct Statics {
    stamina: u32,
    // resource (mana, energy, etc)
    strength: u32,
    agility: u32,
    intelligence: u32,
}