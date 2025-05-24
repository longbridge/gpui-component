# Wef example for Winit

## Table of Contents

1. [How to run](#how-to-run)
    - [MacOS](#macos)
    - [Linux](#linux)
    - [Windows](#windows)

### How to run

#### MacOS

1. Install the required dependencies:

   - [Xcode](https://developer.apple.com/xcode/)
   - [CEF Standard Distribution(x64)](https://cef-builds.spotifycdn.com/index.html#macosx64) or [CEF Standard Distribution(arm64)](https://cef-builds.spotifycdn.com/index.html#macosarm64)
   - Wef tool
     
     ```shell
     cargo install wef-tool
     ```

   - Cargo-bundle
   
     ```shell
     cargo install cargo-bundle
     ```

2. Set the `CEF_ROOT` environment variable to the path of the CEF installation. For example, if you installed CEF in `/path/to/cef`, set `CEF_ROOT` to `/path/to/cef`.

3. Build example

    In the project root directory, run the following command:

    ```bash
    cargo build -p wef-winit-example
    ```

4. Bundle the application

    ```
    cargo bundle -p wef-winit-example
    ```

5. Add helper processes to the application bundle

    ```bash
    wef-tool add-helper target/debug/wef-winit-example/wef-winit-example.app
    ```

    > If you expect release build, you can use `wef-tool add-helper --release target/release/wef-winit-example/wef-winit-example.app` instead.

5. Add Cef Framework to the application bundle

    ```bash
    wef-tool add-framework target/debug/wef-winit-example/wef-winit-example.app
    ```

    > If you expect release build, you can use `wef-tool add-framework --release target/release/wef-winit-example/wef-winit-example.app` instead.

6. Run the example

    ```bash
    open target/debug/wef-winit-example/wef-winit-example.app
    ```

#### Linux

1. Install the required dependencies:

   - [CEF Standard Distribution](https://cef-builds.spotifycdn.com/index.html#linux64)

2. Set the `CEF_ROOT` environment variable to the path of the CEF installation. For example, if you installed CEF in `/path/to/cef`, set `CEF_ROOT` to `/path/to/cef`.

3. Build example

    In the project root directory, run the following command:

    ```bash
    cargo build -p wef-winit-example
    ```

4. Copy the required CEF files to the target directory:

    ```bash
    cp $CEF_ROOT/Debug/*.so target/debug/
    cp $CEF_ROOT/Debug/vk_swiftshader_icd.json target/debug/
    cp $CEF_ROOT/Debug/v8_context_snapshot.bin target/debug/
    cp -r $CEF_ROOT/Resources/* target/debug/
    ```

    > If you expect release build, you can use `cargo build --release` and copy the files from the `Release` directory instead of `Debug`.

5. Run the example

    ```bash
    target/debug/wef-winit-example
    ```

#### Windows

1. Install the required dependencies:

   - [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   - [CEF Standard Distribution](https://cef-builds.spotifycdn.com/index.html#windows64)

2. Set the `CEF_ROOT` environment variable to the path of the CEF installation. For example, if you installed CEF in `C:\cef`, set `CEF_ROOT` to `C:\cef`.

3. Build example

    In the project root directory, run the following command:

    ```bash
    cargo build -p wef-winit-example
    ```

4. Copy the required CEF files to the target directory:

    ```bash
    cp $CEF_ROOT/Debug/*.dll target/debug/
    cp $CEF_ROOT/Debug/vk_swiftshader_icd.json target/debug/
    cp $CEF_ROOT/Debug/v8_context_snapshot.bin target/debug/
    cp -r $CEF_ROOT/Resources/* target/debug/
    ```

    > If you expect release build, you can use `cargo build --release` and copy the files from the `Release` directory instead of `Debug`.

5. Run the example

    ```bash
    .\target\debug\wef-winit-example.exe
    ```
