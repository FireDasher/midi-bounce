import mido
import struct
from tkinter import filedialog

def midi_to_bin(input_file_path: str, output_file_path: str):
	mid = mido.MidiFile(input_file_path)
	times = []
	current_time = 0.0
	last_time = -676767.67
	for msg in mid:
		current_time += msg.time
		if msg.type == "note_on" and msg.velocity > 0 and current_time - last_time >= 0.01:
			last_time = current_time
			times.append(current_time)
	with open(output_file_path, "wb") as f:
		for time in times:
			f.write(struct.pack("<f", time))

if __name__ == "__main__":
	midi_to_bin(filedialog.askopenfilename(), filedialog.asksaveasfilename())