### Game Boy emulator
## Accuracy
* Cart type 0: Fully implemented all games work to my knowledge.
* Cart type 1: Not tested likely very functional.
* Cart type 2: Same as type 1.
* Cart type 3: Cpu correct, error in lcd emulation in **some** games breaks display.
* Cart type 5/6: Working but with lcd start issues.
* Other cart types: Some can boot with the wrong mbc type but don't get much further than that.

## Screenshots
<img width="640" height="577" alt="image" src="https://github.com/user-attachments/assets/f2976d1c-979e-4baf-b265-01d2a1d5b5ea" />

## Usage
Either launch the executable as is or from your terminal
```bash
./gb_emu2
```

# Controls
Arrow key -> dpad
Enter -> start
W -> a
Q -> b
E -> select
F5 -> save state
F9 -> load state

Controllers are also supported if you want to use one instead.
