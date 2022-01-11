extends Control

var initial_report_text: String = "";
var reporter = preload("res://lib/reporter.gdns").new();
var game_name: String = "My Cool Game"; # put your game name here, make sure that it's shorter than 50 characters
var game_version: String = "v1.0.0"; # put your game version here, make sure that it's shorter than 50 characters

func _ready():
	reporter.set_server(127, 0, 0, 1, 50123); # should be according to your server's info
	initial_report_text = get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text;

func send_report(
		report_name: String, report_text: String,
		sender_name: String, sender_email: String):
	var result_code: int = reporter.send_report(
		report_name, report_text, sender_name, sender_email, game_name, game_version);
	if result_code != 0: # if result_code == 0 then everything is OK and the server received your report, otherwise:
		# (it's up to you whether you want to handle all error codes or not, you could ignore the result code or just check if it's equal to 0)
		# (by handling all possible result codes you can provide more information to your user)
		# error code names are taken from /reporter/src/misc.rs
		if result_code == 1:
			# you forgot to call reporter.set_server()
			var error_message: String = "An error occurred: set_server() should be called first.";
			get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = error_message;
			return;
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
			var error_message: String = "The field \"";
			match int(reporter.get_last_error()):
				0:
					error_message += "Summary";
				1:
					error_message += "Issue report";
				2:
					error_message += "Your name";
				3:
					error_message += "Your email";
				4:
					error_message += "game name";
				5:
					error_message += "game version";
			error_message += "\" is too long!";
			get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = error_message;
			return;
		elif result_code == 3:
			# could not connect to the server
			# use "reporter.get_last_error()" to get the error description
			var error_message: String = "Could not connect to the server, error: " + reporter.get_last_error();
			get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = error_message;
			return;
		elif result_code == 4:
			# internal reporter error
			# use "reporter.get_last_error()" to get the error description
			# notify the user and show him the error code so he can report this issue
			# make sure to include "reporter.log" which is located
			# Linux: in the folder with the game,
			# Windows: in the Documents folder, in the subfolder "FBugReporter".
			var error_message: String = "Internal error: " + reporter.get_last_error();
			error_message += "\nPlease, contact the developers of the game and tell them about this issue!\n";
			error_message += "Make sure to include the file \"reporter.log\" which is located at ";
			error_message += reporter.get_log_file_path();
			get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = error_message;
			return;
		elif result_code == 5:
			# wrong protocol
			# the versions (protocols) of the server and the reporter are different (incompatible)
			# if you installed an update of the reporter, make sure you've updated the server (and the client) too
			var error_message: String = "An error occurred: wrong protocol, reporter/server are incompatible!\n";
			error_message += "If you installed an update of the reporter, make sure you've updated the server (and the client) too.";
			get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = error_message;
			return;
		elif result_code == 6:
			# server rejected your report
			# this should probably never happen unless you've modified the source code of the reporter in an incorrect way
			var error_message: String = "An error occurred: the server rejected your report.\n";
			error_message += "This should probably never happen unless you've modified the source code of the reporter in an incorrect way.";
			get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = error_message;
			return;
		elif result_code == 7:
			# network issue
			# something went wrong while transferring the report over the internet
			# and the server received modified/corrupted report
			# tip: try again
			var error_message: String = "An error occurred: network issue.\n";
			error_message += "Something went wrong while transferring the report over the internet and the server received modified/corrupted report.\n";
			error_message += "Try again.";
			get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = error_message;
			return;
	else:
		get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = "We successfully received your report! Thank you!";
		get_node("VBoxContainer/ReportNameHBoxContainer/ReportNameLineEdit").text = "";
		get_node("VBoxContainer/SenderNameHBoxContainer/SenderNameLineEdit").text = "";
		get_node("VBoxContainer/SenderEMailHBoxContainer/SenderEMailLineEdit").text = "";
		get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text = initial_report_text;

func _notification(what):
	if what == NOTIFICATION_PREDELETE:
		# destructor logic
		reporter.queue_free();


func _on_SendReportButton_pressed():
	# Check essential fields.
	if get_node("VBoxContainer/ReportNameHBoxContainer/ReportNameLineEdit").text.length() == 0:
		get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = "Please add a summary.";
	elif initial_report_text == get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text:
		get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = "Please provide an issue description.";
	else:
		get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = "";
		
	# Send report.
	send_report(
		get_node("VBoxContainer/ReportNameHBoxContainer/ReportNameLineEdit").text,
		get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text,
		get_node("VBoxContainer/SenderNameHBoxContainer/SenderNameLineEdit").text,
		get_node("VBoxContainer/SenderEMailHBoxContainer/SenderEMailLineEdit").text);

# text edit character limit
var current_text = ''
var cursor_line = 0
var cursor_column = 0
func _on_ReportTextTextEdit_text_changed():
	if get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text.length() > 5120:
		get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text = current_text;
		get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").cursor_set_line(cursor_line)
		get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").cursor_set_column(cursor_column)
	else:
		current_text = get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text;
		cursor_line = get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").cursor_get_line()
		cursor_column = get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").cursor_get_column()
