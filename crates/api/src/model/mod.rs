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
        .plug::<identity::Key>()
        .plug::<Mirror>()
        .plug::<Email>()
        .plug::<Team>()
        .plug::<Topic>()
        .plug::<Repo>()
        .plug::<Ref>()
        .plug::<Weld>()
        .plug::<Verdict>()
        .plug::<build::Label>()
        .plug::<repo::Label>()
        .plug::<repo::Runner>()
        .plug::<repo::Secret>()
        .plug::<Project>()
        .plug::<Column>()
        .plug::<Release>()
        .plug::<Shield>()
        .plug::<Package>()
        .plug::<build::Runner>()
        .plug::<Run>()
        .plug::<build::Secret>()
        .plug::<Variable>()
        .plug::<build::Key>()
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
