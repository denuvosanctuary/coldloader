# ColdLoader

This project aims to provide a DLL that can be used to load the coldclient version of [GBE](https://github.com/Detanup01/gbe_fork) without needing an external executable.

## Usage

1. **Build the DLL:**

    ```sh
    cargo build --release
    ```

    The resulting DLL will be located in `target/release/coldloader.dll`.

2. **Add the loader files to the game files:**

    Place the compiled `coldloader.dll` in the game files, and add the `coldloader.EXAMPLE.ini` file, renaming it to `coldloader.ini` and filling in the required fields.
    The loader can work without the `coldloader.ini` file, supposed that `steam_settings/steam_appid.txt` contains the App ID and `steamclient64.dll` is present in the same directory.

3. **Add any DLL loader that loads coldloader.dll:**
    
    You can use any DLL loader that loads `coldloader.dll` during the game startup. We recently released [coldloader-proxy](https://github.com/denuvosanctuary/coldloader-proxy) to that does this job! Works with as both version.dll and winmm.dll with no false positives on virustotal.

    [Koaloader](https://github.com/acidicoala/Koaloader) is also a suggested option.

4. **Run the game:**
    
    Start the game as you normally would. The DLL will handle loading the coldclient and patching the registry.

## Debug logs

- In debug builds, logs are written to a timestamped file (`coldloader_<timestamp>.log`).

## Builds

Builds are available in the [releases](https://github.com/denuvosanctuary/coldloader/releases) section of the repository. Nighly builds are also available in the [actions](https://github.com/denuvosanctuary/coldloader/actions) section.

## Disclaimer

This project is highly inspired from the coldclient original loader implementation but adapted to a DLL.
This project is for educational and research purposes only. Use responsibly and respect software licenses.
