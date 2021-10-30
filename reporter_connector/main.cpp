/***************************************************************************
*   Copyright Aleksandr "Flone" Tretyakov (github.com/Flone-dnb).         *
*   Licensed under the MIT license.                                       *
*   Refer to the LICENSE file included.                                   *
***************************************************************************/

#include <string>
#include <iostream>
#include <optional>
#include <variant>
#include <filesystem>
#include <thread>

#ifdef _WIN32
// Winsock 2
#pragma comment(lib,"Ws2_32.lib")
#define FSocket SOCKET
#include <winsock2.h>
#include <ws2tcpip.h>
#include <Windows.h>
#elif __linux__
#define FSocket int
#include <stdlib.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <netdb.h>
#include <errno.h>
#include <unistd.h>
#include <string.h>
#include <spawn.h>
#include <netinet/tcp.h>
#define SOCKET_ERROR -1
#define INVALID_SOCKET -1
#define SD_SEND SHUT_WR
#endif

enum ANSWER_CODE{
    AC_OK = 0,
    AC_WRONG_PROTOCOL = 1,
};

enum REPORT_FIELD{
    RF_REPORT_NAME = 0,
    RF_REPORT_TEXT = 1,
    RF_SENDER_NAME = 2,
    RF_SENDER_EMAIL = 3,
    RF_GAME_NAME = 4,
    RF_GAME_VERSION = 5,
};

enum REPORT_FIELD_LIMIT{
    RFL_REPORT_NAME_BYTES = 100,
    RFL_REPORT_TEXT_BYTES = 5120,
    RFL_SENDER_NAME_BYTES = 100,
    RFL_SENDER_EMAIL_BYTES = 100,
    RFL_GAME_NAME_BYTES = 100,
    RFL_GAME_VERSION_BYTES = 100,
};

constexpr unsigned short CLIENT_PORT = 61234;
constexpr unsigned short REPORTER_PROTOCOL = 0;
constexpr size_t RETRY_CONNECT_COUNT = 5;
constexpr size_t SLEEP_TIME_MS = 1000;

struct GameReport{
    std::string report_name;
    std::string report_text;
    std::string sender_name;
    std::string sender_email;
    std::string game_name;
    std::string game_version;
    // if adding new fields update:
    //  - REPORT_FIELD enum,
    //  - REPORT_FIELD_LIMIT enum,
    //  - REPORTER_PROTOCOL,
    //  - check_fields_limit().
};

std::variant<std::string, ANSWER_CODE> send_report(GameReport&&);
std::optional<std::string> send_data(FSocket, GameReport&&);
bool close_socket(FSocket);
int get_last_error();
void set_socket_nodelay(FSocket);
int reporter(GameReport&& report);
std::optional<std::string> start_reporter();
std::optional<REPORT_FIELD> check_fields_limit(GameReport&);

int main()
{
    GameReport game_report = GameReport();
    game_report.report_name = u8"Мой крутой репорт";
    game_report.report_text = u8"Это мой крутой репорт, вот необычный символ: 仮";
    game_report.sender_name = u8"Александр";
    game_report.sender_email = u8"flonednb@gmail.com";
    game_report.game_name = u8"TestGame";
    game_report.game_version = u8"v1.0.0";

    reporter(std::move(game_report));

    return 0;
}

// returns result to the user code
// positive value means ANSWER_CODE
// -1 = internal error, use get_error_info() for more information
// -2 = report field has an incorrect size, use get_incorrect_field_info() for more information
int reporter(GameReport&& report){
    // Check fields limit.
    std::optional<REPORT_FIELD> limit_result = check_fields_limit(report);
    if (limit_result.has_value()){
        std::cout<<"Field with ID "<<limit_result.value()<<" has wrong size."<<std::endl;
        return -2;
    }

    // Start reporter.
    std::optional<std::string> start_result = start_reporter();
    if (start_result.has_value()){
        std::cout<<start_result.value()<<std::endl;
        return -1;
    }

    // Send report.
    std::variant<std::string, ANSWER_CODE> result = send_report(std::move(report));

    // See result.
    if (std::get_if<ANSWER_CODE>(&result)){
        switch(std::get<ANSWER_CODE>(result)){
            case ANSWER_CODE::AC_OK:
            std::cout<<"All good.\n";
            break;

            case ANSWER_CODE::AC_WRONG_PROTOCOL:
            std::cout<<"Wrong protocol version\n";
            break;
        }

        return std::get<ANSWER_CODE>(result);
    }else{
        // An error occurred:
        std::cout<<std::get<std::string>(result)<<std::endl;

        return -1;
    }
}

std::optional<REPORT_FIELD> check_fields_limit(GameReport& report){
    if (report.report_name.size() > REPORT_FIELD_LIMIT::RFL_REPORT_NAME_BYTES){
        return REPORT_FIELD::RF_REPORT_NAME;
    }

    if (report.report_text.size() > REPORT_FIELD_LIMIT::RFL_REPORT_TEXT_BYTES){
        return REPORT_FIELD::RF_REPORT_TEXT;
    }

    if (report.sender_name.size() > REPORT_FIELD_LIMIT::RFL_SENDER_NAME_BYTES){
        return REPORT_FIELD::RF_SENDER_NAME;
    }

    if (report.sender_email.size() > REPORT_FIELD_LIMIT::RFL_SENDER_EMAIL_BYTES){
        return REPORT_FIELD::RF_SENDER_EMAIL;
    }

    if (report.sender_email.size() > REPORT_FIELD_LIMIT::RFL_SENDER_EMAIL_BYTES){
        return REPORT_FIELD::RF_SENDER_EMAIL;
    }

    if (report.game_name.size() > REPORT_FIELD_LIMIT::RFL_GAME_NAME_BYTES){
        return REPORT_FIELD::RF_GAME_NAME;
    }

    if (report.game_version.size() > REPORT_FIELD_LIMIT::RFL_GAME_VERSION_BYTES){
        return REPORT_FIELD::RF_GAME_VERSION;
    }

    return {};
}

std::optional<std::string> start_reporter(){
#ifdef _WIN32
    std::string current_path = std::filesystem::current_path().string();
    if (current_path[current_path.size() - 1] != '\\'){
        current_path += '\\';
    }
    current_path += "reporter.exe";

    if (!std::filesystem::exists(current_path)){
        std::string msg = std::string("An error occurred at [");
        msg += __FILE__;
        msg += ", ";
        msg += std::to_string(__LINE__);
        msg += "]: reporter binary does not exist.";
        return msg;
    }

    STARTUPINFO si;
    PROCESS_INFORMATION pi;

    ZeroMemory( &si, sizeof(si) );
    si.cb = sizeof(si);
    ZeroMemory( &pi, sizeof(pi) );

    if (CreateProcessA(NULL, static_cast<LPSTR>(const_cast<char*>("reporter.exe")), NULL, NULL, TRUE, 0, NULL, NULL, (LPSTARTUPINFOA)&si, &pi) == 0){
        std::string msg = std::string("An error occurred at [");
        msg += __FILE__;
        msg += ", ";
        msg += std::to_string(__LINE__);
        msg += "]: ";
        msg += std::to_string(GetLastError());
        return msg;
    }
#elif __linux__
    std::string current_path = std::filesystem::current_path().string();
    if (current_path[current_path.size() - 1] != '/'){
        current_path += '/';
    }
    current_path += "reporter";

    if (!std::filesystem::exists(current_path)){
        std::string msg = std::string("An error occurred at [");
        msg += __FILE__;
        msg += ", ";
        msg += std::to_string(__LINE__);
        msg += "]: reporter binary does not exist.";
        return msg;
    }

    pid_t pid;
    char *argv[] = {(char *) 0};
    int status;
    status = posix_spawn(&pid, "./reporter", NULL, NULL, argv, environ);
    if (status != 0) {
        std::string msg = std::string("An error occurred at [");
        msg += __FILE__;
        msg += ", ";
        msg += std::to_string(__LINE__);
        msg += "]: ";
        msg += strerror(status);
        return msg;
    }
#endif

    // Wait for it to start...
    std::this_thread::sleep_for(std::chrono::milliseconds(SLEEP_TIME_MS));

    return {};
}

std::variant<std::string, ANSWER_CODE> send_report(GameReport&& report){
    unsigned short answer_code = 0;

    for (size_t i = 0; i < RETRY_CONNECT_COUNT; i++){
#ifdef _WIN32
    // Start Winsock2.
    WSADATA WSAData;
    WSAStartup(MAKEWORD(2, 2), &WSAData);
#endif

    addrinfo hints;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family   = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_protocol = IPPROTO_TCP;

    addrinfo *addr_info_result = nullptr;


    std::string address = "localhost";
    std::string port_str = std::to_string(CLIENT_PORT);
    int return_code = getaddrinfo(address.c_str(), port_str.c_str(), &hints, &addr_info_result);
    if ( return_code != 0 )
    {
        std::string msg = std::string("An error occurred at [");
        msg += __FILE__;
        msg += ", ";
        msg += std::to_string(__LINE__);
        msg += "]: " + std::to_string(return_code);
        return msg;
    }

    FSocket socket = ::socket(AF_INET, SOCK_STREAM, 0);
    return_code = connect(socket, addr_info_result->ai_addr, addr_info_result->ai_addrlen);

    freeaddrinfo(addr_info_result);

    if ( return_code == SOCKET_ERROR ){
        if (i == RETRY_CONNECT_COUNT - 1){
            return_code = get_last_error();
            std::string msg = std::string("An error occurred at [");
            msg += __FILE__;
            msg += ", ";
            msg += std::to_string(__LINE__);
            msg += "]: " + std::to_string(return_code);
            return msg;
        }else{
            // Try again later.
            std::this_thread::sleep_for(std::chrono::milliseconds(SLEEP_TIME_MS));
            continue;
        }
    }

    set_socket_nodelay(socket);

    std::optional<std::string> result = send_data(socket, std::move(report));
    if ( result.has_value() ){
        return result.value();
    }

    size_t received_count = recv(socket, reinterpret_cast<char*>(&answer_code), sizeof (answer_code), 0);
    if ( received_count != sizeof (answer_code) )
    {
        std::string msg = std::string("An error occurred at [");
        msg += __FILE__;
        msg += ", ";
        msg += std::to_string(__LINE__);
        msg += "]: received ";
        msg += std::to_string(received_count);
        msg += " while expected ";
        msg += sizeof (answer_code);
        return msg;
    }

    // Finish connection.
    shutdown(socket, SD_SEND);
    close_socket(socket);

#ifdef _WIN32
    WSACleanup();
#endif
    }

    return static_cast<ANSWER_CODE>(answer_code);
}

bool close_socket(FSocket socket)
{
#if _WIN32
    if (closesocket(socket))
    {
        return true;
    }
    else
    {
        return false;
    }
#elif __linux__
    if (close(socket))
    {
        return true;
    }
    else
    {
        return false;
    }
#endif
}

std::optional<std::string> send_protocol_version(FSocket socket){
    unsigned short ver = REPORTER_PROTOCOL;

    // Send length of the text.
    size_t sent_bytes = send(socket, reinterpret_cast<char*>(&ver), sizeof (ver), 0);
    if (sent_bytes != sizeof (ver)){
        std::string msg = std::string("An error occurred at [");
        msg += __FILE__;
        msg += ", ";
        msg += std::to_string(__LINE__);
        msg += "]: sent ";
        msg += std::to_string(sent_bytes);
        msg += " while expected ";
        msg += sizeof (ver);
        return msg;
    }

    return {};
}

std::optional<std::string> send_string(FSocket socket, std::string text){
    unsigned short len = text.size();

    size_t sent_bytes = send(socket, reinterpret_cast<char*>(&len), sizeof (len), 0);
    if (sent_bytes != sizeof (len)){
        std::string msg = std::string("An error occurred at [");
        msg += __FILE__;
        msg += ", ";
        msg += std::to_string(__LINE__);
        msg += "]: sent ";
        msg += std::to_string(sent_bytes);
        msg += " while expected ";
        msg += sizeof (len);
        return msg;
    }

    return {};
}

std::optional<std::string> send_data(FSocket socket, GameReport&& report){
    std::optional<std::string> result = send_protocol_version(socket);
    if (result.has_value()){
        return result;
    }

    result = send_string(socket, report.report_name);
    if (result.has_value()){
        return result;
    }

    result = send_string(socket, report.report_text);
    if (result.has_value()){
        return result;
    }

    result = send_string(socket, report.sender_name);
    if (result.has_value()){
        return result;
    }

    result = send_string(socket, report.sender_email);
    if (result.has_value()){
        return result;
    }

    result = send_string(socket, report.game_name);
    if (result.has_value()){
        return result;
    }

    result = send_string(socket, report.game_version);
    if (result.has_value()){
        return result;
    }

    return {};
}

int get_last_error()
{
#if _WIN32
    return WSAGetLastError();
#elif __linux__
    return errno;
#endif
}

void set_socket_nodelay(FSocket socket)
{
    // Disable Nagle algorithm.

#if _WIN32
    BOOL bOptVal = true;
    int bOptLen = sizeof(BOOL);
#elif __linux__
    int bOptVal = 1;
    int bOptLen = sizeof(bOptVal);
#endif
    setsockopt(socket, IPPROTO_TCP, TCP_NODELAY, reinterpret_cast<char*>(&bOptVal), bOptLen);
}
