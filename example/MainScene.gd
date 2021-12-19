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
        # error code names are taken from /reporter/src/misc.rs
		if result_code == 1:
			# you forgot to call reporter.set_server()
			pass;
        elif result_code == 2:
            # invalid input (some input string is too long),
            # use "int(reporter.get_last_error())" to get the id of the invalid field:
            # 0 - report name (limit 50 characters)
            # 1 - report text (limit 5120 characters)
            # 2 - sender name (limit 50 characters)
            # 3 - sender email (limit 50 characters)
            # 4 - game name (limit 50 characters)
            # 5 - game version (limit 50 characters)
            # field ids and limits are taken from /reporter/src/misc.rs
            pass;
        elif result_code == 3:
            # could not connect to the server
            # use "reporter.get_last_error()" to get the error description
            pass;
        elif result_code == 4:
            # internal error
            # use "reporter.get_last_error()" to get the error description
            # notify the user and show him the error code so he can report this issue
            # make sure to include "reporter.log" which is located
            # Linux: in the folder with the game,
            # Windows: in the Documents folder, in the subfolder "FBugReporter".
            pass;

func _notification(what):
	if what == NOTIFICATION_PREDELETE:
		# destructor logic
		reporter.queue_free();
