//! Wiki reading, search and editing for the Flutter app (§8.1 Wiki, §8.3 reader and editor).

use daftar_core::search;

use super::library::LibraryHandle;

pub struct PageSummary {
    pub path: String,
    pub vault: String,
    pub kind: String,
    pub title_en: String,
    pub title_fa: String,
    pub summary: String,
    pub updated: String,
}

pub struct SearchHit {
    pub page: PageSummary,
    /// A line of the page around the first match, as written.
    pub snippet: String,
}

pub struct FolderEntry {
    pub path: String,
    pub pages: u32,
}

pub struct Listing {
    pub folders: Vec<FolderEntry>,
    pub pages: Vec<PageSummary>,
}

pub struct GraphNode {
    pub page: PageSummary,
    pub depth: u32,
}

pub struct GraphEdge {
    pub from: String,
    pub to: String,
}

pub struct LocalGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// A page in the whole-wiki graph, with the number of distinct pages it is linked with.
pub struct WikiGraphNode {
    pub page: PageSummary,
    pub links: u32,
}

/// The whole wiki as a graph. Edges hold indexes into `nodes`, one per linked pair.
pub struct WikiGraph {
    pub nodes: Vec<WikiGraphNode>,
    pub edge_from: Vec<u32>,
    pub edge_to: Vec<u32>,
}

pub struct WikiPage {
    pub path: String,
    /// The whole file, for the source editor.
    pub text: String,
    /// Markdown after the frontmatter, for the renderer.
    pub body: String,
    pub hash: String,
    pub kind: String,
    pub vault: String,
    pub title_en: String,
    pub title_fa: String,
    pub aliases: Vec<String>,
    pub summary: String,
    pub updated: String,
    pub status: String,
    pub source_count: u32,
    pub backlinks: Vec<PageSummary>,
}

pub struct SaveResult {
    pub hash: String,
    pub committed: bool,
}

fn err(e: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}

fn summary(r: search::PageRef) -> PageSummary {
    PageSummary {
        path: r.path,
        vault: r.vault,
        kind: r.kind,
        title_en: r.title_en,
        title_fa: r.title_fa,
        summary: r.summary,
        updated: r.updated,
    }
}

impl LibraryHandle {
    pub fn search(
        &self,
        query: String,
        vaults: Vec<String>,
        limit: u32,
    ) -> anyhow::Result<Vec<SearchHit>> {
        let hits = self
            .session()
            .search(&query, &vaults, limit as usize)
            .map_err(err)?;
        Ok(hits
            .into_iter()
            .map(|h| SearchHit {
                snippet: h.snippet,
                page: PageSummary {
                    path: h.path,
                    vault: h.vault,
                    kind: h.kind,
                    title_en: h.title_en,
                    title_fa: h.title_fa,
                    summary: h.summary,
                    updated: h.updated,
                },
            })
            .collect())
    }

    pub fn recent_pages(
        &self,
        vault: Option<String>,
        limit: u32,
    ) -> anyhow::Result<Vec<PageSummary>> {
        Ok(self
            .session()
            .recent_pages(vault.as_deref(), limit as usize)
            .map_err(err)?
            .into_iter()
            .map(summary)
            .collect())
    }

    /// One level of the page tree, e.g. `vaults/life` or `vaults/life/people`.
    pub fn list_dir(&self, dir: String) -> anyhow::Result<Listing> {
        let l = self.session().list_dir(&dir).map_err(err)?;
        Ok(Listing {
            folders: l
                .folders
                .into_iter()
                .map(|f| FolderEntry {
                    path: f.path,
                    pages: f.pages as u32,
                })
                .collect(),
            pages: l.pages.into_iter().map(summary).collect(),
        })
    }

    pub fn local_graph(&self, path: String, depth: u32) -> anyhow::Result<LocalGraph> {
        let g = self
            .session()
            .local_graph(&path, depth as usize)
            .map_err(err)?;
        Ok(LocalGraph {
            nodes: g
                .nodes
                .into_iter()
                .map(|n| GraphNode {
                    page: summary(n.page),
                    depth: n.depth as u32,
                })
                .collect(),
            edges: g
                .edges
                .into_iter()
                .map(|(from, to)| GraphEdge { from, to })
                .collect(),
        })
    }

    /// Every page and the links between them, optionally in one vault (the Graph view).
    pub fn wiki_graph(&self, vault: Option<String>) -> anyhow::Result<WikiGraph> {
        let g = self.session().wiki_graph(vault.as_deref()).map_err(err)?;
        let (edge_from, edge_to) = g.edges.iter().map(|&(a, b)| (a as u32, b as u32)).unzip();
        Ok(WikiGraph {
            nodes: g
                .nodes
                .into_iter()
                .map(|n| WikiGraphNode {
                    page: summary(n.page),
                    links: n.links as u32,
                })
                .collect(),
            edge_from,
            edge_to,
        })
    }

    pub fn page(&self, path: String) -> anyhow::Result<WikiPage> {
        let v = self.session().page(&path).map_err(err)?;
        Ok(WikiPage {
            path: v.path,
            text: v.text,
            body: v.body,
            hash: v.hash,
            kind: v.meta.kind,
            vault: v.meta.vault,
            title_en: v.meta.title.en,
            title_fa: v.meta.title.fa,
            aliases: v.meta.aliases,
            summary: v.meta.summary,
            updated: v.meta.updated,
            status: v.meta.status,
            source_count: v.meta.sources.len() as u32,
            backlinks: v.backlinks.into_iter().map(summary).collect(),
        })
    }

    /// Where a wikilink points (`sara`, `people/sara`, `raw/2026/09/23/…`), if it exists.
    pub fn resolve_link(&self, target: String) -> anyhow::Result<Option<String>> {
        self.session().resolve_link(&target).map_err(err)
    }

    /// Saves an edit as one human commit; fails if the page changed since `base_hash`.
    pub fn save_page(
        &self,
        path: String,
        base_hash: String,
        text: String,
    ) -> anyhow::Result<SaveResult> {
        let o = self
            .session()
            .save_page(&path, &base_hash, &text)
            .map_err(err)?;
        Ok(SaveResult {
            hash: o.hash,
            committed: o.committed,
        })
    }

    /// Picks up files changed outside the app (on resume and after sync).
    pub fn refresh_index(&self) -> anyhow::Result<u32> {
        Ok(self.session().refresh_index().map_err(err)? as u32)
    }

    /// Settings › Repository › Rebuild index.
    pub fn rebuild_index(&self) -> anyhow::Result<u32> {
        Ok(self.session().rebuild_index().map_err(err)? as u32)
    }
}
