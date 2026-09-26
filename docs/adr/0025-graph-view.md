# ADR-0025: The Graph view

* Status: accepted
* Date: 2026-09-26

## Context

The page reader already had **Nearby pages**, a ring of neighbours up to two links away. People
who come from Obsidian also expect a graph of the whole wiki, so they can see clusters, hubs and
pages that nothing links to.

## Decisions

**One query for the whole graph.** `SearchIndex::wiki_graph` reads every page and every link row
once, then resolves each link with one `Resolver`. Calling `local_graph` for each page would be
O(n²). Each linked pair becomes one edge, whichever way the link points. Nodes carry their link
count. Edges travel over FFI as two `u32` index arrays, not as path strings. Generated vault indexes
are left out because they link to everything, and so are raw citations. Both rules come from the
index.

**Filters run on the device.** The app fetches the graph once per index revision and applies the
vault filter and the unlinked-pages filter locally. When a filter changes, nodes that stay on
screen keep their positions, so the picture doesn't jump. The core still takes an optional vault
for the CLI (`daftar graph <repo> [--vault v]`).

**Our own layout, no dependency.** `graph_layout.dart` is a d3-force style simulation:

* springs along links;
* Barnes–Hut repulsion, so a 5,000-page wiki stays interactive;
* a weak pull towards the centre that keeps clusters and unlinked pages in view;
* phyllotaxis seeding, so the layout is deterministic and goldens are stable.

The graph packages on pub.dev are either unmaintained or bring Material chrome. The simulation is
about 200 lines and is unit-tested.

**Only the features that help you find your way:**

* pan, pinch or scroll to zoom, and fit to screen;
* drag a node, and its neighbours follow;
* tap a node to light up its neighbours and show a card with Open; tap it again to open the page;
* hover to preview the same highlight on desktop;
* find a page by name;
* chips filter by vault and double as the colour legend (`GraphHues` tokens);
* hide unlinked pages.

Labels fade in as you zoom, and well-linked pages get labels first. We left out Obsidian's physics
sliders, groups and time-lapse on purpose.

**Entry points.** A graph button in the Wiki tab header opens the graph. **Open the graph** in a
page's Nearby pages sheet opens it centred on that page (`/wiki/graph?focus=…`).

With reduced motion, the layout jumps straight to its settled state and the camera doesn't
animate. The canvas has a spoken summary ("12 pages · 18 links"). The search list is the accessible
way to reach any page.
