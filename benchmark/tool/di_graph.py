#!/usr/bin/env python3
"""Generate a synthetic, fully reachable FURNACE dependency graph."""

from __future__ import annotations


def parse_dependency_counts(raw: str) -> tuple[int, ...]:
    values = raw.split(",")
    if not values or any(not value.strip() for value in values):
        raise ValueError("dependency counts must be comma-separated positive integers")
    try:
        counts = tuple(int(value.strip()) for value in values)
    except ValueError as error:
        raise ValueError("dependency counts must be comma-separated positive integers") from error
    if any(count < 1 for count in counts):
        raise ValueError("dependency counts must be positive")
    if len(set(counts)) != len(counts):
        raise ValueError("dependency counts must be unique")
    return counts


def dependency_layout(node_count: int) -> tuple[tuple[int, ...], ...]:
    if node_count < 1:
        raise ValueError("dependency count must be positive")
    return tuple(
        tuple(child for child in (2 * index + 1, 2 * index + 2) if child < node_count)
        for index in range(node_count)
    )


def render_source(node_count: int, declarative: bool) -> str:
    layout = dependency_layout(node_count)
    nodes: list[str] = []
    implementations: list[str] = []
    marker = "#[inject] " if declarative else ""
    for index, children in enumerate(layout):
        name = f"Dependency{index:04}"
        if children:
            fields = ",\n".join(
                f"    {marker}child_{position}: Dependency{child:04}"
                for position, child in enumerate(children)
            )
            nodes.append(f"#[burner]\nstruct {name} {{\n{fields},\n}}")
            count = " + ".join(
                f"self.child_{position}.dependency_count()"
                for position in range(len(children))
            )
            implementations.append(
                f"impl {name} {{\n    fn dependency_count(&self) -> usize {{\n"
                f"        touch_graph_node();\n        1 + {count}\n    }}\n}}"
            )
        else:
            nodes.append(f"#[burner]\nstruct {name};")
            implementations.append(
                f"impl {name} {{\n    fn dependency_count(&self) -> usize {{\n"
                "        touch_graph_node();\n        1\n    }\n}"
            )

    provider_names = ", ".join(f"Dependency{index:04}" for index in range(node_count))
    if declarative:
        cauldron = (
            "#[cauldron(root, providers = [" + provider_names
            + "], controllers = [GraphController])]\nstruct AppCauldron;"
        )
    else:
        registration = "\n            ".join(
            f".provide::<Dependency{index:04}>()" for index in range(node_count)
        )
        cauldron = (
            "#[cauldron]\nstruct AppCauldron;\n\n"
            "impl furnace_rs::core::Cauldron for AppCauldron {\n"
            "    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {\n"
            "        self\n            " + registration + "\n"
            "            .controller::<GraphController>()\n"
            "    }\n"
            "}"
        )

    return "\n\n".join([
        "use furnace_rs::prelude::*;\n"
        "use std::sync::atomic::{AtomicUsize, Ordering};\n"
        "static GRAPH_VISITS: AtomicUsize = AtomicUsize::new(0);\n"
        "fn touch_graph_node() { GRAPH_VISITS.fetch_add(1, Ordering::Relaxed); }",
        *nodes,
        *implementations,
        f"#[controller]\nstruct GraphController {{ {marker}root: Dependency0000 }}\n\n"
        "#[controller(route = \"/graph\")]\nimpl GraphController {\n"
        "    #[get(\"/noop\")]\n"
        "    async fn noop(&self) -> &'static str { \"ok\" }\n"
        "    #[get(\"/traverse\")]\n"
        "    async fn dependency_count(&self) -> String {\n"
        "        self.root.dependency_count().to_string()\n"
        "    }\n"
        "}",
        cauldron,
        "#[furnace_rs::main]\nasync fn main() -> Result<(), HttpRuntimeError> {\n"
        "    Furnace::burn::<AppCauldron>().await\n"
        "}",
        "",
    ])
