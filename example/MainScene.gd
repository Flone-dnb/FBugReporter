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
	if result_code != 0: # if result_code == 0 then everything is OK and the server received your report, otherwise:
        # (it's up to you whether you want to handle all error codes or not, you could ignore the result code or just check if it's equal to 0)
        # (by handling all possible result codes you can provide more information to your user)
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
            # internal reporter error
            # use "reporter.get_last_error()" to get the error description
            # notify the user and show him the error code so he can report this issue
            # make sure to include "reporter.log" which is located
            # Linux: in the folder with the game,
            # Windows: in the Documents folder, in the subfolder "FBugReporter".
            pass;
        elif result_code == 5:
            # wrong protocol
            # the versions (protocols) of the server and the reporter are different (incompatible)
            # if you installed an update of the reporter, make sure you've updated the server (and the client) too
            pass;
        elif result_code == 6:
            # server rejected your report
            # this should probably never happen unless you've modified the source code of the reporter in the incorrect way
            pass;
        elif result_code == 7:
            # network issue
            # something went wrong while transferring the report over the internet
            # and the server received modified/corrupted report
            # tip: try again
            pass;

func _notification(what):
	if what == NOTIFICATION_PREDELETE:
		# destructor logic
		reporter.queue_free();
