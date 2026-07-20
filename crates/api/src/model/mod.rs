mod build;
mod identity;
mod issue;
mod repo;

pub(crate) use build::*;
pub(crate) use identity::*;
pub(crate) use issue::*;
pub(crate) use repo::*;

use keel::Graph;

keel_gate::gate!(Actor);
keel_relay::relay!(Actor);
keel_blob::blob!(Actor);

pub(crate) fn shape() -> Graph {
    let mut graph = Graph::new();
    graph
        .plug::<Actor>()
        .plug::<UserKey>()
        .plug::<Mirror>()
        .plug::<Email>()
        .plug::<Team>()
        .plug::<Topic>()
        .plug::<Repo>()
        .plug::<Label>()
        .plug::<OrgLabel>()
        .plug::<OrgRunner>()
        .plug::<OrgSecret>()
        .plug::<Project>()
        .plug::<Column>()
        .plug::<Release>()
        .plug::<Shield>()
        .plug::<Package>()
        .plug::<Runner>()
        .plug::<Run>()
        .plug::<Secret>()
        .plug::<Variable>()
        .plug::<Key>()
        .plug::<Milestone>()
        .plug::<Issue>()
        .plug::<Comment>()
        .plug::<Reaction>()
        .plug::<Pull>()
        .plug::<Review>()
        .plug::<Note>();
    plug(&mut graph);
    wire(&mut graph);
    stock(&mut graph);
    graph
}
