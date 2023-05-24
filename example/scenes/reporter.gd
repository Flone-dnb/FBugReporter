extends Node3D

# -----------------------------------------------------------------------------

@onready var reporter: FBugReporter = get_node("FBugReporter")
@onready var result_text_node: Label = get_node("UI/Background/VBoxContainer/ResultText")
@onready var report_text_node: TextEdit =  get_node("UI/Background/VBoxContainer/ReportBox/ReportTextBox")
@onready var report_name_node: LineEdit = get_node("UI/Background/VBoxContainer/SummaryBox/SummaryTextBox")
@onready var sender_name_node: LineEdit = get_node("UI/Background/VBoxContainer/NameBox/NameTextBox")
@onready var sender_email_node: LineEdit = get_node("UI/Background/VBoxContainer/EmailBox/EmailTextBox")
@onready var screenshot_option_box: OptionButton = get_node("UI/Background/VBoxContainer/ScreenshotBox/ScreenshotOptionBox")

const game_name: String = "mygame" # put your game name here, make sure that it's shorter than 50 characters
const game_version: String = "v1.0.0" # put your game version here, make sure that it's shorter than 50 characters

var report_text_limit: int = 0; # stores the maximum size of the report contents
var initial_report_text: String = ""; # stores initial contents of the "report" text box

# -----------------------------------------------------------------------------

func _ready():
	reporter.setup_game(game_name, game_version);
	reporter.setup_report_receiver("Server", "127.0.0.1:50123", "");
	
	initial_report_text = report_text_node.text;

	# set length limits
	report_name_node.max_length = reporter.get_field_limit("ReportName");
	sender_name_node.max_length = reporter.get_field_limit("SenderName");
	sender_email_node.max_length = reporter.get_field_limit("SenderEmail");
	report_text_limit = reporter.get_field_limit("ReportText");
	
	# print limits
	print("report text limit is " + str(report_text_limit))

func send_report(
		report_name: String, # report name
		report_text: String, # report text
		sender_name: String, # optional sender name
		sender_email: String, # optional sender email
		report_attachments, # an array of strings that contain paths to the files to attach to report, it's safer to specify absolute paths
		take_screenshot: bool): # whether to send game screenshot or not 
	# Set report data.
	reporter.set_report_name(report_name);
	reporter.set_report_text(report_text);
	reporter.set_sender_name(sender_name);
	reporter.set_sender_email(sender_email);
	reporter.set_report_attachments(report_attachments);

	# Take a screenshot.
	if take_screenshot:
		# Hide our widget (reporter UI).
		get_node("UI").visible = false;

		# Wait until the frame has finished before getting the texture.
		await RenderingServer.frame_post_draw

		# Retrieve the viewport texture (image).
		var img: Image = get_viewport().get_texture().get_image();

		# Show our widget.
		get_node("UI").visible = true;

		# Draw another frame with our widget back.
		await RenderingServer.frame_post_draw

		# Scale image.
		img.resize(1920, 1080);

		# Save screenshot.
		reporter.set_screenshot(img);
	else:
		# Remove previously saved screenshot (if there was one).
		reporter.set_clear_screenshot();

	# Send report.
	var result_code: int = reporter.send_report();
	var error_message: String = ""
	if result_code != 0: # if result_code == 0 then everything is OK and the server received your report, otherwise:
		# (it's up to you whether you want to handle all error codes or not, you could ignore the result code or just check if it's equal to 0)
		# (by handling all possible result codes you can provide more information to your user)
		# error code names are taken from server/shared/src/misc/report.rs (see `ReportResult` enum)
		if result_code == 1:
			# you forgot to initialize the reporter
			error_message = "Remote address / report receiver type is not set.";
		elif result_code == 2:
			# invalid input (some input string is too long)
			error_message = "The field \"" + reporter.get_last_error() + "\" is too long!";
		elif result_code == 3:
			# could not connect to the server
			error_message = "Could not connect to the server.";
		elif result_code == 4:
			# the specified attachment(s) do not exist, check that all path to the attachments are valid
			error_message = "The specified attachment(s) do not exist.";
		elif result_code == 5:
			# the specified attachments exceed the maximum allowed attachment size limit on the server

			if take_screenshot:
				# Try again without a screenshot.
				push_warning("the specified report attachments exceed the maximum allowed report attachment size limit on the server " +
					"attempting to resend without a screenshot");
				send_report(report_name, report_text, sender_name, sender_email, report_attachments, false);
				return;

			error_message = "The specified attachments exceed the maximum allowed attachment size limit.";
		elif result_code == 6:
			# other error, use `get_last_error` for description
			error_message = "An error occurred: " + str(reporter.get_last_error()) + "."
		else:
			# adding this just in case
			error_message = "The reporter returned unknown error code \"" + str(result_code) + "\".";
	
	# show result message
	if result_code == 0:
		result_text_node.text = "We successfully received your report! Thank you!";

		# clear all fields, with this you can't send the same report again with just another button press
		report_name_node.text = "";
		sender_name_node.text = "";
		sender_email_node.text = "";
		screenshot_option_box.selected = 0
		report_text_node.text = initial_report_text;
	else:
		error_message += "\nIf you're not the developer of this game, please, contact the developers and tell them about this issue!\n";
		error_message += "Make sure to include the file \"reporter.log\" which is located at ";
		error_message += reporter.get_log_file_path();

		result_text_node.text = error_message

func _on_SendReportButton_pressed():
	# Make sure report summary is specified.
	if report_name_node.text.length() == 0:
		result_text_node.text = "Please add a summary.";
		return;
	
	# Make sure report text is valid.
	if initial_report_text == report_text_node.text || report_text_node.text.length() == 0:
		result_text_node.text = "Please fill issue description sections that start with '#' with actual information.";
		return;
	
	# Update UI.
	result_text_node.text = "Sending your report. Please wait...";
	await RenderingServer.frame_post_draw; # wait for one frame to be drawn

   # Add 3 most recent log files as attachments.
	var log_path = OS.get_user_data_dir() + "/logs";
	var last_logs = reporter.get_last_modified_files(log_path, 3); # returns N most recent files

	var attach_screenshot = screenshot_option_box.get_selected_id() == 1;

	# Send report.
	send_report(
		report_name_node.text,
		report_text_node.text,
		sender_name_node.text,
		sender_email_node.text,
		last_logs,
		attach_screenshot);

# text edit character limit
var current_text = ''
var cursor_line = 0
var cursor_column = 0
func _on_ReportTextTextEdit_text_changed():
	# Limit the number of characters that the user can enter:
	if report_text_node.text.length() > report_text_limit:
		report_text_node.text = current_text;
		report_text_node.set_caret_line(cursor_line)
		report_text_node.set_caret_column(cursor_column)
	else:
		current_text = report_text_node.text;
		cursor_line = report_text_node.get_caret_line()
		cursor_column = report_text_node.get_caret_column()
