# Usage

## Prism Launcher

To inject Tuxinjector into Minecraft while using [Prism Launcher](https://prismlauncher.org/), you need to set a **Wrapper Command** in its settings settings.

### Steps

1. Open Prism Launcher and select your Minecraft instance.
2. Click **Settings** in the sidebar.
3. Go to the **Custom Commands** tab.
4. Check **Override Global Settings** if it isn't already enabled.
5. In the **Wrapper Command** field, enter:

    ```
    env LD_PRELOAD=/path/to/tuxinjector.so
    ```

    Replace `/path/to/tuxinjector.so` with the actual path to the built library on your system.

6. Launch the instance normally.

![Image of Prism Launcher Custom Commands tab, showing the wrapper command](images/wrapper-prism.png)

!!! tip
    Unlike [waywall](https://github.com/tesselslate/waywall) which uses `waywall --wrap` as the wrapper command to launch a nested compositor, Tuxinjector injects **directly** into the game process via `LD_PRELOAD`. The `env` command simply sets the environment variable that tells the dynamic linker to load Tuxinjector's shared library into Java before Minecraft starts.


!!! note
    You can also set this globally under **Settings > Custom Commands** in Prism Launcher's main window, which will apply to all instances.

!!! note
    You can also set the under the Environment Variables tab, by setting the name to `LD_PRELOAD`, and the value as the path to your .so file.

<!-- TODO: Update this for MCSR Launcher, hopefully by using the Tools Tab just like how toolscreen does it ^_^ --> 
