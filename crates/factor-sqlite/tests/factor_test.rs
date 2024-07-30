use std::{collections::HashSet, sync::Arc};

use factor_sqlite::{runtime_config::spin::SpinSqliteRuntimeConfig, SqliteFactor};
use spin_factors::{
    anyhow::{self, bail, Context},
    runtime_config::toml::TomlKeyTracker,
    Factor, FactorRuntimeConfigSource, RuntimeConfigSourceFinalizer, RuntimeFactors,
};
use spin_factors_test::{toml, TestEnvironment};

#[derive(RuntimeFactors)]
struct TestFactors {
    sqlite: SqliteFactor,
}

#[tokio::test]
async fn sqlite_works() -> anyhow::Result<()> {
    let test_resolver = DefaultLabelResolver::new(Some("default"));
    let factors = TestFactors {
        sqlite: SqliteFactor::new(test_resolver),
    };
    let env = TestEnvironment::new(factors).extend_manifest(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        sqlite_databases = ["default"]
    });
    let state = env.build_instance_state().await?;

    assert_eq!(
        state.sqlite.allowed_databases(),
        &["default".into()].into_iter().collect::<HashSet<_>>()
    );

    Ok(())
}

#[tokio::test]
async fn errors_when_non_configured_database_used() -> anyhow::Result<()> {
    let test_resolver = DefaultLabelResolver::new(None);
    let factors = TestFactors {
        sqlite: SqliteFactor::new(test_resolver),
    };
    let env = TestEnvironment::new(factors).extend_manifest(toml! {
        [component.test-component]
        source = "does-not-exist.wasm"
        sqlite_databases = ["foo"]
    });
    let Err(err) = env.build_instance_state().await else {
        bail!("Expected build_instance_state to error but it did not");
    };

    assert!(err
        .to_string()
        .contains("One or more components use SQLite databases which are not defined."));

    Ok(())
}

#[tokio::test]
async fn no_error_when_database_is_configured() -> anyhow::Result<()> {
    let test_resolver = DefaultLabelResolver::new(None);
    let factors = TestFactors {
        sqlite: SqliteFactor::new(test_resolver),
    };
    let runtime_config = toml! {
        [sqlite_database.foo]
        type = "spin"
    };
    let sqlite_config = SpinSqliteRuntimeConfig::new("/".into(), "/".into());
    let env = TestEnvironment::new(factors)
        .extend_manifest(toml! {
            [component.test-component]
            source = "does-not-exist.wasm"
            sqlite_databases = ["foo"]
        })
        .runtime_config(TomlRuntimeSource::new(&runtime_config, sqlite_config))?;
    env.build_instance_state()
        .await
        .context("build_instance_state failed")?;
    Ok(())
}

struct TomlRuntimeSource<'a> {
    table: TomlKeyTracker<'a>,
    sqlite_config: SpinSqliteRuntimeConfig,
}

impl<'a> TomlRuntimeSource<'a> {
    fn new(table: &'a toml::Table, sqlite_config: SpinSqliteRuntimeConfig) -> Self {
        Self {
            table: TomlKeyTracker::new(table),
            sqlite_config,
        }
    }
}

impl FactorRuntimeConfigSource<SqliteFactor> for TomlRuntimeSource<'_> {
    fn get_runtime_config(
        &mut self,
    ) -> anyhow::Result<Option<<SqliteFactor as Factor>::RuntimeConfig>> {
        self.sqlite_config.config_from_table(&self.table)
    }
}

impl RuntimeConfigSourceFinalizer for TomlRuntimeSource<'_> {
    fn finalize(&mut self) -> anyhow::Result<()> {
        Ok(self.table.validate_all_keys_used().unwrap())
    }
}

impl TryFrom<TomlRuntimeSource<'_>> for TestFactorsRuntimeConfig {
    type Error = anyhow::Error;

    fn try_from(value: TomlRuntimeSource<'_>) -> Result<Self, Self::Error> {
        Self::from_source(value)
    }
}

/// Will return an `InvalidConnectionPool` for the supplied default database.
struct DefaultLabelResolver {
    default: Option<String>,
}

impl DefaultLabelResolver {
    fn new(default: Option<&str>) -> Self {
        Self {
            default: default.map(Into::into),
        }
    }
}

impl factor_sqlite::DefaultLabelResolver for DefaultLabelResolver {
    fn default(&self, label: &str) -> Option<Arc<dyn factor_sqlite::ConnectionPool>> {
        let Some(default) = &self.default else {
            return None;
        };
        (default == label).then_some(Arc::new(InvalidConnectionPool))
    }
}

/// A connection pool that always returns an error.
struct InvalidConnectionPool;

#[async_trait::async_trait]
impl factor_sqlite::ConnectionPool for InvalidConnectionPool {
    async fn get_connection(
        &self,
    ) -> Result<Arc<dyn factor_sqlite::Connection + 'static>, spin_world::v2::sqlite::Error> {
        Err(spin_world::v2::sqlite::Error::InvalidConnection)
    }
}