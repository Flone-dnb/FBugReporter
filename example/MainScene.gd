extends Control

var reporter = preload("res://lib/reporter.gdns").new();

func _ready():
	reporter.set_server(127, 0, 0, 1, 50123); # should be according to your server's info

func send_report(
		report_name: String, report_text: String,
		sender_name: String, sender_email: String,
		game_name: String, game_version: String):
	var result_code: int = reporter.send_report(
		report_name, report_text, sender_name, sender_email, game_name, game_version);
	if result_code != 0:
		if result_code == 1:
			# you forgot to call reporter.set_server()
			pass;
		else:
			# notify the user and show him the error code so he can report this issue
			# make sure to include "FBugReporter - reporter.log" which is located
			# Linux: in the folder with the game,
			# Windows: in the Documents folder, in the subfolder "FBugReporter".
			pass

func _notification(what):
	if what == NOTIFICATION_PREDELETE:
		# destructor logic
		reporter.queue_free();
