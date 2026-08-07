//! The arrow every 3D vector in the scene is drawn with.
//!
//! An arrow is a parent entity holding two children: a shaft stretched along
//! `+Y` and a head at its tip. The parent carries the direction, so pointing
//! one is a rotation and sizing one is [`lay_out`].

use bevy::prelude::*;

/// The stem of an arrow. Stretched along Y to the vector's length.
#[derive(Component)]
pub struct ArrowShaft;

/// The cone of an arrow. Moved to the tip, but kept at its built size.
#[derive(Component)]
pub struct ArrowHead {
    pub height: f32,
}

/// Shafts, disjoint from the heads and from the `Owner` that groups them.
pub type ShaftQuery<'w, 's, Owner> =
    Query<'w, 's, &'static mut Transform, (With<ArrowShaft>, Without<ArrowHead>, Without<Owner>)>;

/// Heads, disjoint from the shafts and from the `Owner` that groups them.
pub type HeadQuery<'w, 's, Owner> = Query<
    'w,
    's,
    (&'static ArrowHead, &'static mut Transform),
    (Without<ArrowShaft>, Without<Owner>),
>;

/// Lay the shaft and the head out along an arrow of `world_length`.
///
/// The two are placed end to end rather than the parent being scaled in Y:
/// that stretched the cone into a needle on long vectors and squashed it flat
/// on short ones. The head keeps its built size, shrinking only when the arrow
/// is too short to hold it — and then uniformly, so it stays a cone. The tip
/// lands exactly at `world_length`.
pub fn lay_out<Owner: Component>(
    children: &Children,
    world_length: f32,
    shafts: &mut ShaftQuery<Owner>,
    heads: &mut HeadQuery<Owner>,
) {
    let head_height = children
        .into_iter()
        .find_map(|&child| heads.get(child).ok().map(|(head, _)| head.height))
        .unwrap_or(0.0);
    let head_length = head_height.min(world_length * 0.5);
    let shaft_length = world_length - head_length;

    for &child in children {
        if let Ok(mut shaft) = shafts.get_mut(child) {
            shaft.scale.y = shaft_length.max(1e-4);
            shaft.translation.y = shaft_length * 0.5;
        } else if let Ok((head, mut head_transform)) = heads.get_mut(child) {
            head_transform.scale = Vec3::splat(head_length / head.height);
            head_transform.translation.y = shaft_length + head_length * 0.5;
        }
    }
}
