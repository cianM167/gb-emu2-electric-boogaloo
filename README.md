# Game Boy emulator
## Accuracy
* Mbc1-Mbc3 are fully implemented games with them are fully playable,
* There are some issues with timing on mbc3 games but this can be addressed by boosting clock speeds
* Rom clock and batteries are not emulated so saves dont work you need to use save states
* audio is incomplete



## Screenshots
<img width="640" height="577" alt="image" src="https://github.com/user-attachments/assets/f2976d1c-979e-4baf-b265-01d2a1d5b5ea" />
<img width="640" height="575" alt="image" src="https://github.com/user-attachments/assets/97569bf0-a639-4335-a225-7bbe69464d62" />
<img width="640" height="575" alt="image" src="https://github.com/user-attachments/assets/6031eca8-1306-4052-8c5e-ddc61cb7e133" />



## Usage
Either launch the executable as is or from your terminal
```bash
./gb_emu2
```
### args
* `--headless`: Runs the emulator with no visual output
* `--record <filename>`: Records your inputs to the specified file
* `--frames`: Specify how many frames the emulator should run for before closing
* `--input-script <filename>`: Give the emulator inputs to be played back
* `--trace-out <filename>`: Logs cpu state to the provided file
* `--custom-cart <filename>`: Allows user to load custom mapper logic to be used with the emulator, incomplete but will eventually allow for emulation of external audio hardware

### Controls
* Arrow key -> dpad
* Enter -> start
* W -> a
* Q -> b
* E -> select
* F2 -> Slow down
* F3 -> Speed up
* F5 -> save state
* F9 -> load state

Controllers are also supported if you want to use one instead.
