### Dr Driving PC
a port of dr driving made to run on machines other than android.

<img width="1920" height="1080" alt="2026-11-08_08-18-56" src="https://github.com/user-attachments/assets/3024d773-4650-4b7a-b979-55bb4f76bc92" />
<img width="1920" height="1080" alt="2026-11-08_08-17-32" src="https://github.com/user-attachments/assets/25268686-1329-492a-8041-90d556ffef05" />


### Installation
download the binaries for your operating system from the [releases](https://github.com/KrishnaSSH/drdriving-pc/releases/latest) 

### Build from Source
make sure your system has rust toolchain installed
you can install it from [rustup.rs](https://rustup.rs)

```
cargo run --release -p drdriving
```

### Game data files

##### Linux
```
$XDG_DATA_HOME/drdriving/save.json
```
will use
```
~/.local/share/drdriving/save.json
```

if that variable isn't set


##### Windows
```
 %APPDATA%\drdriving\save.json
```

##### MacOS

```
~/Library/Application Support/drdriving/save.json
```



### Discord Rich Presence

`DRDRIVING_NO_RPC=1` to turn off the discord rpc.

### Local sandbox: unlimited coins, gold and unlocked cars

This local build starts with unlimited coins and gold (shown as `MAX`) and all 28 selectable
cars unlocked. Rental cars never expire. The bus remains an AI vehicle and the truck is used
by the Truck mission. Upgrades still follow the normal driver-level requirements.

Sandbox progress uses `%APPDATA%\drdriving\sandbox-save.json`, separate from the regular save.
Set `DRDRIVING_UNLIMITED_COINS=0` before starting to use normal progression and `save.json`.
Online matchmaking is disabled in sandbox mode and whenever custom car models are installed.

PowerShell:

```powershell
cd "C:\path\to\drdriving-pc"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:DRDRIVING_UNLIMITED_COINS = "1"
cargo run --release -p drdriving
```

On Windows, you can also double-click `Play-Unlimited.cmd`. It opens the project directory,
selects the Rustup toolchain, builds the game if needed, and starts the local sandbox.

### Import a custom car (Windows)

Open the garage, select the car slot to replace with the left/right arrows, then click
**Import car (OBJ + PNG)**. Choose the OBJ first and its PNG texture second. The replacement
appears immediately and reloads on future launches. **Restore original** removes the replacement
for that slot. The original game assets are preserved.

Prepare a complete low-poly car, including its wheels, with Y up and the front facing +Z.
Export a triangulated Wavefront OBJ with UV coordinates, using one PNG texture atlas.
Limits: 20,000 triangles, 8 MB per file, and a texture no larger than 2048 x 2048 pixels.
Materials, animations, separate texture maps, FBX and GLTF are not supported by this importer.

The model is centered, placed on the ground, and uniformly scaled to fit the selected stock car.
Handling, collisions, and cockpit remain those of the selected slot. Optional `carNN.name` files
beside installed replacements provide their display names. A triangulated OBJ can animate its
wheels when they are separate objects named `wheel_fl`, `wheel_fr`, `wheel_rl`, and `wheel_rr`;
include a separate body object and all four wheels. Untagged custom wheels remain static.
Stock wheels and brake-light meshes are hidden for custom models. Replacing a slot
also changes traffic vehicles that use that slot. Models are stored locally in
`%APPDATA%\drdriving\custom-cars\`, without sending files to a server.

### Car list, animated wheels and driving sounds

Click **Car list (L)** in the garage or mission selection, or press **L**. Installed custom cars
appear first, followed by the original cars. Click a row or press its number (1–8); use
Previous/Next or the left/right arrow keys for more pages. All seven installed BMW/modern car
models have four animated wheels. Front wheels steer and the wheels roll backwards in reverse.

Driving audio crossfades between RPM loops as speed and throttle change. The Cybertruck uses
electric motor loops; the BMW i8 mixes motor and combustion audio, while sports cars use a
higher-revving profile. Original cars retain combustion sound. Pause, race exit and the Sound
setting stop driving loops. These are game sound profiles, not recordings of each real vehicle.

For local diagnostics, `--audio-test` exercises all four sound profiles and reports animated
wheel counts. `--car N` chooses a slot (0–27). `--shot FILE --scene carlist` captures the selector;
`--shot FILE --scene garage --car N --wheel-phase DEGREES` captures wheel animation poses.

### Detailed car rendering and showroom

The seven locally installed modern cars now have `.carhq` packages, loaded before the older
single-texture `.car` packages. These preserve full source geometry, smooth normals, separate
material textures (up to their original resolution, capped at 4096), and four animated wheels.
The new car shader adds roughness/metallic shading, normal maps where supplied, clearcoat,
tinted glass, filtered environment reflections, and soft contact shadows. Source models range
from roughly 5,000 to 138,000 triangles; the McLaren, Cybertruck and BMW 320i source models
remain less detailed than the others. This renderer does not provide photorealism or ray tracing.

In the garage, click **Showroom (V)** or press **V** for a larger studio view. Drag or use
**A/D/W/S** to orbit, scroll or use **+/-** to zoom, and press **R** to reset the view.
Previous/Next selects another car; **Escape** returns to the garage. The same detailed models
are used in driving scenes. `--shot FILE --scene showroom --car N` captures this view.

`Install-Detailed-Cars.ps1` reinstalls the generated local pack, backing up any existing detailed
packages. **Restore original** in the garage removes both replacement formats for that slot;
importing a new OBJ replaces its detailed package. Original game assets are preserved.
See [the car-quality tools](tools/car-quality/README.md) for sources and rebuild instructions.

### Browser version

GitHub Pages custom domain: [Play Dr. Driving — Modern Garage](https://dr-driving.meowsy.space/).

The WebAssembly/WebGL build bundles all seven detailed cars, unlimited coins/gold, the car
selector, showroom and driving sounds. Progress saves locally in each browser. The Windows
OBJ/PNG file picker remains a desktop feature. Model and game credits are included in
`CREDITS.md` and `LICENSE.txt` in the browser bundle. The sandbox does not connect to online matchmaking.

On phones and tablets, **Auto** selects Performance: 1× screen resolution without MSAA,
lighter car geometry and reflections, and smaller mirror targets. Desktop Auto keeps
the full-detail renderer. Open the sliders button in-game to choose Auto, Performance or
High quality; applying a change saves it and reloads.
Geometry for the city and stock cars is cached instead of rebuilt every frame.

The screen button requests browser fullscreen where supported. Where it is unavailable,
it shows Home Screen instructions. On iPhone, open the game in Safari, use **Share → Add to
Home Screen**, enable **Open as Web App** if shown, and launch the new icon. The manifest
opens it as a standalone app; notch/home-indicator insets and changing browser-bar heights
are reserved around the game canvas. Turn the phone sideways for the intended driving layout.

With Rustup and Node.js installed:

```powershell
rustup target add wasm32-unknown-unknown
node web/build.mjs
node web/serve.mjs
```

Open `http://127.0.0.1:4185/`. Output in `web/dist/` can be served by static hosting.
Use `node web/build.mjs --no-build` to package an existing release WASM. Set
`DRDRIVING_SITE_URL` to the deployed HTTPS base URL (including the repository path for
GitHub Pages) to populate social-preview URLs.
Car packages in `web/car-pack/` are separate from `assets.bin` so each file stays under 25 MiB.
Mobile packages are generated with `node tools/car-quality/export-mobile-cars.mjs` after
installing the dependencies in `tools/car-quality/`. They retain source materials and wheel
pivots, cap textures at 1024px, and simplify dense meshes within an appearance-error budget.
`node web/validate.mjs` and `node web/validate-display.mjs` check package integrity and browser
bridge/capability fixtures. These checks do not measure FPS on physical phones.

The Pages workflow builds and validates the browser bundle on every push to `main`.
In repository **Settings → Pages**, the publishing source is **GitHub Actions**.
The custom domain is recorded in `web/CNAME` and copied into the published bundle.
DNS: `dr-driving` is a CNAME to `kushagr1501.github.io`.

### Visitor analytics

[Vercel Analytics dashboard](https://vercel.com/kushagr1501s-projects/dr-driving-analytics/analytics)
tracks visits, referrers, countries, devices and browsers for `dr-driving.meowsy.space`.
The game stays on GitHub Pages. A small Vercel project in `web/analytics-collector/` supplies
the official analytics script and a collection endpoint with CORS for the game domain.
Deploy that folder separately after changing its configuration; normal game updates use Pages.

`web/analytics.js` loads asynchronously on the production domain, skips local/preview sites
and browser privacy opt-outs, and removes query values/fragments from recorded page URLs.
Analytics failures do not block the game. No player names, save data, car uploads, game inputs,
or custom user identifiers are sent. The integration does not call Vercel's identify/group or
cookie APIs. The current Hobby plan uses page views; custom events such as Play clicks require
Pro and are not enabled here. Older traffic cannot be recovered.

Run `node web/validate-analytics.mjs` for loader/queue fixtures. Verify the live `/collect/view`
endpoint and dashboard separately; fixture checks do not prove events reached Vercel.
The setup smoke test records one page view at `/__analytics_verification`; this is a test,
not a player visit. The actual home page remains `/` in reports.
