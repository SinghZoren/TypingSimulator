import tkinter as tk
import threading
import pyautogui
import time
import random
import keyboard  

typing_thread = None
typing_active = False 
text_to_type = ""     

def start_typing():
    """
    Main function that handles the "realistic" typing:
      - Splits text into words.
      - Waits 3s before starting to allow user to switch windows.
      - Types each word at ~120 WPM with random pauses.
      - Occasionally types a wrong letter and backspaces to fix it.
    """
    global typing_active

    time.sleep(3)

    words = text_to_type.split()

    for word in words:
        if not typing_active:
            break

        for letter in word:
            if not typing_active:
                break

            if random.random() < 0.10:
                wrong_char = random.choice("abcdefghijklmnopqrstuvwxyz")
                pyautogui.typewrite(wrong_char) 
                time.sleep(0.08)  
                pyautogui.press('backspace')
                time.sleep(0.08)

            pyautogui.typewrite(letter)  
            time.sleep(0.0001) 

        if typing_active:
            pyautogui.typewrite(" ")
            time.sleep(0.0001)
          
        if typing_active:
            time.sleep(random.uniform(0.0001, 0.005))

    typing_active = False


def toggle_typing():
    """
    Called by the F6 hotkey.
    If typing is not active, start a new thread to type the text.
    If typing is active, stop it.
    """
    global typing_active, typing_thread

    if not typing_active:
        gui_text = text_box.get("1.0", tk.END).rstrip('\n')

        if not gui_text.strip():
            return 

        set_text_to_type(gui_text)
        typing_active = True
        typing_thread = threading.Thread(target=start_typing)
        typing_thread.daemon = True 
        typing_thread.start()
    else:
        typing_active = False

def set_text_to_type(text):
    """ Store the text to be typed into the global variable. """
    global text_to_type
    text_to_type = text

def on_close():
    """
    Gracefully handle when the user closes the Tkinter window:
      - Stop typing if active
      - Unregister hotkey
      - Destroy window
    """
    global typing_active
    typing_active = False

    try:
        keyboard.remove_hotkey('f6')
    except Exception:
        pass

    root.destroy()


root = tk.Tk()
root.title("Realistic Typing Emulator (~120 WPM)")

label = tk.Label(root, text="Paste your text below:")
label.pack(pady=5)

text_box = tk.Text(root, height=5, width=50)
text_box.pack(pady=5)

info_label = tk.Label(
    root, 
    text=(
        "Press F6 to start/stop typing.\n"
        "After pressing F6, you have 3s to switch windows.\n"
        "Typing speed is ~120 WPM with random typos."
    )
)
info_label.pack(pady=10)

def register_hotkey():
    """
    Register the F6 hotkey with the keyboard library.
    If F6 is pressed, it toggles typing.
    """
    keyboard.add_hotkey('f6', toggle_typing)

root.protocol("WM_DELETE_WINDOW", on_close)
root.after(100, register_hotkey)

root.mainloop()
