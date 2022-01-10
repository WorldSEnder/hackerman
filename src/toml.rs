use guppy::{graph::PackageGraph, PackageId};
use std::{collections::BTreeMap, path::Path};
use toml_edit::{table, value, Array, Document, InlineTable, Item, Table, Value};
use tracing::debug;

fn to_table<'a>(toml: &'a mut Document, path: &[&str]) -> anyhow::Result<&'a mut Table> {
    let mut entry = toml
        .entry(path[0])
        .or_insert_with(table)
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("Expected table"))?;
    entry.set_implicit(true);
    for comp in &path[1..] {
        entry = entry
            .entry(comp)
            .or_insert_with(table)
            .as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("Expected table"))?;
        entry.set_implicit(true);
    }
    Ok(entry)
}

pub fn set_dependencies<P>(
    manifest_path: P,
    g: &PackageGraph,
    patch: &BTreeMap<&PackageId, Vec<&str>>,
) -> anyhow::Result<()>
where
    P: AsRef<Path> + std::fmt::Debug,
{
    let kind = "dependencies";
    let mut toml = std::fs::read_to_string(&manifest_path)?.parse::<Document>()?;

    if !toml["package"]["metadata"]["hackerman"][kind].is_none() {
        anyhow::bail!(
            "{:?} already contains changes, restore the original files before applying a new hack",
            manifest_path
        );
    }

    let table = to_table(&mut toml, &[kind])?;
    let mut changes = Vec::new();
    for (package_id, feats) in patch.iter() {
        let dep = g.metadata(package_id)?;
        let name = dep.name();

        let mut new_dep = InlineTable::new();
        new_dep.insert("version", dep.version().to_string().into());
        let mut feats_arr = Array::new();
        feats_arr.extend(feats.iter().copied());
        new_dep.insert("features", Value::Array(feats_arr));

        changes.push((name, table.insert(name, value(new_dep))));
    }
    table.sort_values();

    let stash_table = to_table(&mut toml, &["package", "metadata", "hackerman", kind])?;
    for (name, old) in changes {
        match old {
            Some(t) => stash_table.insert(name, t),
            None => stash_table.insert(name, value(false)),
        };
    }
    stash_table.sort_values();
    stash_table.set_position(999);

    std::fs::write(&manifest_path, toml.to_string())?;

    Ok(())
}
