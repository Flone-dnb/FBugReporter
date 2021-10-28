/***************************************************************************
*   Copyright Aleksandr "Flone" Tretyakov (github.com/Flone-dnb).         *
*   Licensed under the MIT license.                                       *
*   Refer to the LICENSE file included.                                   *
***************************************************************************/

#include <string>
#include <iostream>
#include <optional>

// Sockets and stuff
#ifdef _WIN32
#ifdef _WIN32
// Winsock 2
#pragma comment(lib,"Ws2_32.lib")
#endif
#define FSocket SOCKET
using std::memcpy;
#include <winsock2.h>
#include <ws2tcpip.h>
#elif __linux__
#define FSocket int
#include <sys/socket.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <netdb.h>
#include <fcntl.h>
#include <errno.h>
#include <unistd.h>
#include <string.h>
#include <time.h>
#include <netinet/tcp.h>
#define SOCKET_ERROR -1
#define INVALID_SOCKET -1
#define SD_SEND SHUT_WR
#endif

enum ANSWER_CODE{
    AC_OK = 0,
};

constexpr unsigned short CLIENT_PORT = 61234;

struct GameReport{
    std::string report_name;
    std::string report_text;
    std::string sender_name;
    std::string sender_email;
    std::string game_name;
    std::string game_version;
};

std::optional<std::string> send_report(GameReport&&);
std::optional<std::string> send_data(FSocket, GameReport&&);
bool close_socket(FSocket);
int get_last_error();
void set_socket_nodelay(FSocket);

int main()
{
    GameReport game_report = GameReport();
    game_report.report_name = u8"Мой крутой репорт";
    game_report.report_text = u8"Это мой крутой репорт, вот необычный символ: 仮";
    game_report.sender_name = u8"Александр";
    game_report.sender_email = u8"flonednb@gmail.com";
    game_report.game_name = u8"TestGame";
    game_report.game_version = u8"v1.0.0";

    std::optional<std::string> result = send_report(std::move(game_report));
    if (result.has_value()){
        std::cout<<result.value()<<std::endl;
    }

    return 0;
}

std::optional<std::string> send_report(GameReport&& report){
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
        return_code = get_last_error();
        std::string msg = std::string("An error occurred at [");
        msg += __FILE__;
        msg += ", ";
        msg += std::to_string(__LINE__);
        msg += "]: " + std::to_string(return_code);
        return msg;
    }

    set_socket_nodelay(socket);

    std::optional<std::string> result = send_data(socket, std::move(report));
    if ( result.has_value() ){
        return result;
    }

    int answer_code = 0;
    size_t received_count = recv(socket, &answer_code, sizeof (answer_code), 0);
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

    // See answer code.
    if (answer_code == ANSWER_CODE::AC_OK){
        // Ok.
    }

    // Finish connection.
    shutdown(socket, SD_SEND);
    close_socket(socket);

#ifdef _WIN32
    WSACleanup();
#endif

    return {};
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

std::optional<std::string> send_string(FSocket socket, std::string text){
    unsigned short len = text.size();

    // Send length of the text.
    size_t sent_bytes = send(socket, &len, sizeof (len), 0);
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

    if (len == 0){
        return {};
    }

    // Send text.
    sent_bytes = send(socket, text.c_str(), len, 0);
    if (sent_bytes != len){
        std::string msg = std::string("An error occurred at [");
        msg += __FILE__;
        msg += ", ";
        msg += std::to_string(__LINE__);
        msg += "]: sent ";
        msg += std::to_string(sent_bytes);
        msg += " while expected ";
        msg += len;
        return msg;
    }

    return {};
}

std::optional<std::string> send_data(FSocket socket, GameReport&& report){
    std::optional<std::string> result = send_string(socket, report.report_name);
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
