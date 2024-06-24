/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/.
 */

use std::fs::File;
use std::io::Write;
use std::{collections::HashMap, fs};

use crate::{BindingGenerator, Component, ComponentInterface};
use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};

pub use gen_kotlin_multiplatform::generate_bindings;

pub mod gen_kotlin_multiplatform;

use gen_kotlin_multiplatform::Config;

mod test;
pub use test::{run_script, run_test};

pub struct KotlinMultiplatformBindings {
    common: String,
    jvm: String,
    native: String,
    header: String,
}

pub struct KotlinMultiplatformBindingGenerator;

impl BindingGenerator for KotlinMultiplatformBindingGenerator {
    type Config = Config;

    fn new_config(&self, root_toml: &toml::Value) -> Result<Self::Config> {
        Ok(
            match root_toml
                .get("bindings")
                .and_then(|b| b.get("kotlin-multiplatform"))
            {
                Some(v) => v.clone().try_into()?,
                None => Default::default(),
            },
        )
    }

    fn update_component_configs(
        &self,
        settings: &crate::GenerationSettings,
        components: &mut Vec<Component<Self::Config>>,
    ) -> Result<()> {
        for c in &mut *components {
            c.config
                .package_name
                .get_or_insert_with(|| format!("uniffi.{}", c.ci.namespace()));
            c.config.cdylib_name.get_or_insert_with(|| {
                settings
                    .cdylib
                    .clone()
                    .unwrap_or_else(|| format!("uniffi_{}", c.ci.namespace()))
            });
        }
        // We need to update package names
        let packages = HashMap::<String, String>::from_iter(
            components
                .iter()
                .map(|c| (c.ci.crate_name().to_string(), c.config.package_name())),
        );
        for c in components {
            for (ext_crate, ext_package) in &packages {
                if ext_crate != c.ci.crate_name()
                    && !c.config.external_packages.contains_key(ext_crate)
                {
                    c.config
                        .external_packages
                        .insert(ext_crate.to_string(), ext_package.clone());
                }
            }
        }
        Ok(())
    }

    fn write_bindings(
        &self,
        settings: &crate::GenerationSettings,
        components: &[Component<Self::Config>],
        // ci: &ComponentInterface,
        // config: &Self::Config,
        // out_dir: &Utf8Path,
    ) -> Result<()> {
        for Component { ci, config, .. } in components {
            let bindings = generate_bindings(config, ci)?;

            create_target(ci, config, &settings.out_dir, "common", bindings.common);
            create_target(ci, config, &settings.out_dir, "jvm", bindings.jvm);
            create_target(ci, config, &settings.out_dir, "native", bindings.native);

            create_cinterop(ci, &settings.out_dir, bindings.header);
        }

        Ok(())
    }
}

fn create_target(
    ci: &ComponentInterface,
    config: &Config,
    out_dir: &Utf8Path,
    name: &str,
    content: String,
) {
    let source_set_name = format!("{}Main", name);
    let package_path: Utf8PathBuf = config.package_name().split(".").collect();
    let file_name = format!("{}.{}.kt", ci.namespace(), name);

    let dst_dir = Utf8PathBuf::from(out_dir)
        .join(&source_set_name)
        .join("kotlin")
        .join(package_path);
    let file_path = Utf8PathBuf::from(&dst_dir).join(file_name);

    fs::create_dir_all(&dst_dir).unwrap();
    let mut f = File::create(&file_path).unwrap();
    write!(f, "{}", content).unwrap();
}

fn create_cinterop(ci: &ComponentInterface, out_dir: &Utf8Path, content: String) {
    let dst_dir = Utf8PathBuf::from(out_dir)
        .join("nativeInterop")
        .join("cinterop")
        .join("headers")
        .join(ci.namespace());
    fs::create_dir_all(&dst_dir).unwrap();
    let file_path = Utf8PathBuf::from(dst_dir).join(format!("{}.h", ci.namespace()));
    let mut f = File::create(&file_path).unwrap();
    write!(f, "{}", content).unwrap();
}
