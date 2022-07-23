extends Control

var initial_report_text: String = "";
var reporter = preload("res://lib/reporter.gdns").new();
var game_name: String = "My Cool Game"; # put your game name here, make sure that it's shorter than 50 characters
var game_version: String = "v1.0.0"; # put your game version here, make sure that it's shorter than 50 characters
var report_text_limit: int = 10;

func _ready():
	# specify your server's IP (or domain name) and port
	# using 'localhost' for local usage (this computer)
	reporter.set_server("localhost", 21580);
	initial_report_text = get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text;
	
	# set game data
	reporter.set_game_name(game_name);
	reporter.set_game_version(game_version);
	
	# set length limits
	# get_field_limit() values are from server/shared/src/report.rs
	get_node("VBoxContainer/ReportNameHBoxContainer/ReportNameLineEdit").max_length = reporter.get_field_limit(0);
	get_node("VBoxContainer/SenderNameHBoxContainer/SenderNameLineEdit").max_length = reporter.get_field_limit(2);
	get_node("VBoxContainer/SenderEMailHBoxContainer/SenderEMailLineEdit").max_length = reporter.get_field_limit(3);
	report_text_limit = reporter.get_field_limit(1);

func send_report(
		report_name: String, # report name
		report_text: String, # report text
		sender_name: String, # optional sender name
		sender_email: String, # optional sender email
		report_attachments, # an array of strings that contain paths to the files to attach to report, it's safer to specify absolute paths
		take_screenshot: bool): # whether to send game screenshot or not 
	# Add last 3 log files as attachments.
	var log_path = OS.get_user_data_dir() + "/logs";
	var last_logs = reporter.get_last_modified_files(log_path, 3);
	report_attachments += last_logs;
	
	# Set report data.
	reporter.set_report_name(report_name);
	reporter.set_report_text(report_text);
	reporter.set_sender_name(sender_name);
	reporter.set_sender_email(sender_email);
	reporter.set_report_attachments(report_attachments);
	
	# Take a screenshot.
	if take_screenshot:
		var old_clear_mode = get_viewport().get_clear_mode();
		get_viewport().set_clear_mode(Viewport.CLEAR_MODE_ONLY_NEXT_FRAME);
		
		# Hide our widget.
		get_node(".").visible = false;
		
		# Wait until the frame has finished before getting the texture.
		yield(VisualServer, "frame_post_draw")
		
		# Retrieve the captured image.
		var img: Image = get_viewport().get_texture().get_data();
		
		get_viewport().set_clear_mode(old_clear_mode);
		
		# Show our widget.
		get_node(".").visible = true;
		
		# Draw another frame with our widget back.
		yield(VisualServer, "frame_post_draw")

		# Flip it on the y-axis (because it's flipped).
		img.flip_y();
		
		# Scale image.
		img.resize(1920, 1080);
		
		# Save screenshot.
		reporter.set_screenshot(img);
	else:
		reporter.set_clear_screenshot();
	
	# Send report.
	var result_code: int = reporter.send_report();
	if result_code != 0: # if result_code == 0 then everything is OK and the server received your report, otherwise:
		# (it's up to you whether you want to handle all error codes or not, you could ignore the result code or just check if it's equal to 0)
		# (by handling all possible result codes you can provide more information to your user)
		# error code names are taken from server/shared/src/report.rs
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
			# field ids and limits are taken from server/shared/src/report.rs
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
			error_message += "This should probably never happen unless you've modified the source code of the reporter in an incorrect way.\n";
			error_message += "If you're not the developer of this game, please, contact the developers and tell them about this issue!\n";
			error_message += "Make sure to include the file \"reporter.log\" which is located at ";
			error_message += reporter.get_log_file_path();
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
		elif result_code == 8:
			# the specified attachment(s) do not exist
			# check that all path to the attachments are valid
			var error_message: String = "An error occurred: the specified attachment(s) do not exist.\n";
			error_message += "Make sure that all specified attached file paths are valid.\n";
			error_message += "If you're not the developer of this game, please, contact the developers and tell them about this issue!\n";
			error_message += "Make sure to include the file \"reporter.log\" which is located at ";
			error_message += reporter.get_log_file_path();
			get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = error_message;
		elif result_code == 9:
			# the specified attachments exceed the maximum allowed attachment size limit on the server
			
			# NOTICE ME: here it's better to try again without some attachment(s)
			var notice_me;
			
			var error_message: String = "An error occurred: the specified attachments exceed the maximum allowed attachment size limit.\n";
			error_message += "If you're not the developer of this game, please, contact the developers and tell them about this issue!\n";
			error_message += "Make sure to include the file \"reporter.log\" which is located at ";
			error_message += reporter.get_log_file_path();
			get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = error_message;
		else:
			# adding this just in case
			var error_message: String = "An error occurred: reporter returned unknown error code \"" + String(result_code) + "\".\n";
			error_message += "If you're not the developer of this game, please, contact the developers and tell them about this issue!\n";
			error_message += "Make sure to include the file \"reporter.log\" which is located at ";
			error_message += reporter.get_log_file_path();
			get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = error_message;
	else:
		# clear all fields, with this
		# you can't send the same report again with just another button press
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
		return;
	elif initial_report_text == get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text || get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text.length() == 0:
		get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = "Please provide a proper issue description.";
		return;
	else:
		get_node("VBoxContainer/SendResultHBoxContainer2/SendResultLabel").text = "Sending your report. Please wait...";
		
	yield(VisualServer, "frame_post_draw"); # wait for one frame to be drawn
	
	var include_screenshot = get_node("VBoxContainer/ScreenshotHBoxContainer/ScreenshotCheckBox").pressed;
	
	# Send report.
	send_report(
		get_node("VBoxContainer/ReportNameHBoxContainer/ReportNameLineEdit").text,
		get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text,
		get_node("VBoxContainer/SenderNameHBoxContainer/SenderNameLineEdit").text,
		get_node("VBoxContainer/SenderEMailHBoxContainer/SenderEMailLineEdit").text,
		[],
		include_screenshot);

# text edit character limit
var current_text = ''
var cursor_line = 0
var cursor_column = 0
func _on_ReportTextTextEdit_text_changed():
	if get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text.length() > report_text_limit:
		get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text = current_text;
		get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").cursor_set_line(cursor_line)
		get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").cursor_set_column(cursor_column)
	else:
		current_text = get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").text;
		cursor_line = get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").cursor_get_line()
		cursor_column = get_node("VBoxContainer/ReportTextHBoxContainer/ReportTextTextEdit").cursor_get_column()
