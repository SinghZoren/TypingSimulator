# TypingSimulator

Automates "realistic" typing of pasted text with a controllable hotkey. The app uses `tkinter` for the UI, `pyautogui` to emit keystrokes, and `keyboard` to register a global hotkey.

---

## Requirements

- Python 3.9 or newer (macOS and Windows both supported)
- Ability to install Python packages via `pip`
- Accessibility permissions to allow simulated keystrokes:
  - **macOS:** enable the Terminal (or your IDE) under *System Settings → Privacy & Security → Accessibility*.
  - **Windows:** run your terminal/IDE as Administrator the first time you use the `keyboard` library.

All Python dependencies are listed in `requirements.txt`.

---

## Setup

### macOS

1. Open Terminal and change into the project directory:
   ```bash
   cd /path/to/TypingSimulator
   ```
2. (Optional) Create and activate a virtual environment:
   ```bash
   python3 -m venv .venv
   source .venv/bin/activate
   ```
3. Install dependencies:
   ```bash
   python3 -m pip install --upgrade pip
   python3 -m pip install -r requirements.txt
   ```
4. Grant Accessibility permissions when prompted (or beforehand, as noted above).

### Windows

1. Open PowerShell and change into the project directory:
   ```powershell
   cd C:\Users\<YourUser>\Desktop\TypingSimulator
   ```
2. (Optional) Create and activate a virtual environment:
   ```powershell
   py -3 -m venv .venv
   .\.venv\Scripts\Activate.ps1
   ```
   > If script execution is restricted, run `Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass` first.
3. Install dependencies:
   ```powershell
   python -m pip install --upgrade pip
   python -m pip install -r requirements.txt
   ```
4. Run PowerShell or your IDE as Administrator the first time so the `keyboard` library can register the hotkey.

---

## Usage

1. Launch the app:
   - macOS: `python3 main.py`
   - Windows: `python main.py`
2. Paste or type the text you want to simulate in the text box.
3. Switch to the target window.
4. Press `F6` to start or stop typing.
   - You have three seconds after pressing `F6` to focus the target window.
   - The script types around 120 WPM, occasionally inserting and correcting typos for realism.
5. Close the UI window or press `F6` again to stop the automation.

---

## Troubleshooting

- **Hotkey not working:** ensure the app has Accessibility/Administrator privileges and no other app is intercepting `F6`.
- **Unexpected typing speed:** adjust timing constants in `start_typing()` inside `main.py`.
- **Unicode characters:** `pyautogui` types basic ASCII reliably; for other character sets results may vary depending on OS keyboard layout.

