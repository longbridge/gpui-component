# Todo App GPUI

This project is a simple Todo application built using the GPUI framework. It allows users to manage their tasks efficiently with a user-friendly interface.

## Features

- **Main Todo View**: Displays a list of todo items with options to add, edit, and delete tasks.
- **Todo Form**: A form for creating and editing todo items.
- **Settings**: An interface for configuring application settings.
- **MCP Configuration**: A form for setting up Model Configuration Provider settings.
- **Provider Configuration**: A form for configuring model service providers.

## Project Structure

```
todo-app-gpui
├── Cargo.toml          # Rust project configuration
├── README.md           # Project documentation
└── src
    ├── main.rs         # Entry point of the application
    ├── models          # Contains data models
    │   ├── mod.rs      # Module declaration for models
    │   ├── todo_item.rs # Definition of TodoItem struct
    │   ├── mcp_config.rs # Definition of MCPConfig struct
    │   └── provider_config.rs # Definition of ProviderConfig struct
    └── views           # Contains view definitions
        ├── mod.rs      # Module declaration for views
        ├── todo_main_view.rs # Main interface for displaying todos
        ├── todo_form.rs # Form for creating/editing todo items
        ├── settings_main_view.rs # Settings interface
        ├── mcp_form.rs  # Form for MCP configuration
        └── provider_form.rs # Form for provider configuration
```

## Usage

1. Clone the repository:
   ```
   git clone <repository-url>
   cd todo-app-gpui
   ```

2. Build the project:
   ```
   cargo build
   ```

3. Run the application:
   ```
   cargo run
   ```

## Contributing

Contributions are welcome! Please open an issue or submit a pull request for any enhancements or bug fixes.

## License

This project is licensed under the MIT License. See the LICENSE file for more details.