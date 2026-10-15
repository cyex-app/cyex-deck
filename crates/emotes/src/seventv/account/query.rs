//! V1051-02 The GQL documents (research/7tv-cosmetics.md §2).

/// Own state and owned items by Twitch id. Needs no token.
pub const INVENTORY: &str = r#"query Cosmetics($tw: String!, $product: Id!) {
  users { userByConnection(platform: TWITCH, platformId: $tw) {
    id
    style { activePaintId activeBadgeId }
    billing(productId: $product) { subscriptionInfo { activePeriod { end } totalDays } }
    inventory(includeInaccessible: true) {
      paints { accessible from { __typename } to { paint { id name updatedAt data {
        layers { id opacity ty { __typename
          ... on PaintLayerTypeSingleColor { color { hex } }
          ... on PaintLayerTypeLinearGradient { angle repeating stops { at color { hex } } }
          ... on PaintLayerTypeRadialGradient { shape repeating stops { at color { hex } } }
          ... on PaintLayerTypeImage { images { url mime scale frameCount } } } }
        shadows { color { hex } offsetX offsetY blur } } } } }
      badges { accessible from { __typename } to { badge { id name description images { url mime scale frameCount } } } }
    } } }
}"#;

/// The signed-in user: id and connections.
pub const ME: &str = "query Me { users { me { id connections { platform platformId } } } }";

pub const SET_PAINT: &str = "mutation SetPaint($user: Id!, $paint: Id) { users { user(id: $user) { activePaint(paintId: $paint) { id style { activePaintId } } } } }";

pub const SET_BADGE: &str = "mutation SetBadge($user: Id!, $badge: Id) { users { user(id: $user) { activeBadge(badgeId: $badge) { id style { activeBadgeId } } } } }";
