use crate::home_tab::HomePage;
use gpui::{Context, Window};
use one_core::storage::{ConnectionType, StoredConnection, Workspace};

pub(crate) trait ConnectionOpenStrategy {
    fn open(self: Box<Self>, home: &mut HomePage, window: &mut Window, cx: &mut Context<HomePage>);
}

pub(crate) fn build_connection_open_strategy(
    connection: StoredConnection,
    workspace: Option<Workspace>,
) -> Box<dyn ConnectionOpenStrategy> {
    match connection.connection_type {
        ConnectionType::SshSftp => Box::new(SshOpenStrategy { connection }),
        ConnectionType::Database => Box::new(DatabaseOpenStrategy {
            connection,
            workspace,
        }),
        ConnectionType::Redis => Box::new(RedisOpenStrategy {
            connection,
            workspace,
        }),
        ConnectionType::MongoDB => Box::new(MongoOpenStrategy {
            connection,
            workspace,
        }),
        ConnectionType::Serial => Box::new(SerialOpenStrategy { connection }),
        _ => Box::new(NoopOpenStrategy),
    }
}

struct SshOpenStrategy {
    connection: StoredConnection,
}

impl ConnectionOpenStrategy for SshOpenStrategy {
    fn open(self: Box<Self>, home: &mut HomePage, window: &mut Window, cx: &mut Context<HomePage>) {
        home.open_ssh_terminal(self.connection, window, cx);
    }
}

struct DatabaseOpenStrategy {
    connection: StoredConnection,
    workspace: Option<Workspace>,
}

impl ConnectionOpenStrategy for DatabaseOpenStrategy {
    fn open(self: Box<Self>, home: &mut HomePage, window: &mut Window, cx: &mut Context<HomePage>) {
        let DatabaseOpenStrategy {
            connection,
            workspace,
        } = *self;
        home.add_item_to_tab(&connection, workspace, window, cx);
    }
}

struct RedisOpenStrategy {
    connection: StoredConnection,
    workspace: Option<Workspace>,
}

impl ConnectionOpenStrategy for RedisOpenStrategy {
    fn open(self: Box<Self>, home: &mut HomePage, window: &mut Window, cx: &mut Context<HomePage>) {
        let RedisOpenStrategy {
            connection,
            workspace,
        } = *self;
        home.open_redis_tab(connection, workspace, window, cx);
    }
}

struct MongoOpenStrategy {
    connection: StoredConnection,
    workspace: Option<Workspace>,
}

impl ConnectionOpenStrategy for MongoOpenStrategy {
    fn open(self: Box<Self>, home: &mut HomePage, window: &mut Window, cx: &mut Context<HomePage>) {
        let MongoOpenStrategy {
            connection,
            workspace,
        } = *self;
        home.open_mongodb_tab(connection, workspace, window, cx);
    }
}

struct NoopOpenStrategy;

struct SerialOpenStrategy {
    connection: StoredConnection,
}

impl ConnectionOpenStrategy for SerialOpenStrategy {
    fn open(self: Box<Self>, home: &mut HomePage, window: &mut Window, cx: &mut Context<HomePage>) {
        home.open_serial_terminal(self.connection, window, cx);
    }
}

impl ConnectionOpenStrategy for NoopOpenStrategy {
    fn open(
        self: Box<Self>,
        _home: &mut HomePage,
        _window: &mut Window,
        _cx: &mut Context<HomePage>,
    ) {
    }
}
