//! The device API of user programs, enabled by the `int_nodem` feature.

use virtmach::interrupts::SoftInterruptFunction;

/// The device API of user programs: interrupt "nodem", the first custom interrupt after virtmach's
/// built-in slots (number interrupts::BUILTIN_SLOTS, i.e. 10).
/// Implemented by the device firmware (nodem-ai-editor: firmware/nodem-vm/src/lib.rs) — keep numbers in sync.
pub static FUNCTIONS: [SoftInterruptFunction; 27] = [
    SoftInterruptFunction { no: 0, name: "wait",        arguments: 1, returns: 0, help: "Pause the program for ms milliseconds, 0 = until the next frame (ms)->()" },
    SoftInterruptFunction { no: 1, name: "button",      arguments: 0, returns: 1, help: "Next button press: 0 none, 1-4 short press of button n, 11-14 long press ()->(key)" },
    SoftInterruptFunction { no: 2, name: "page",        arguments: 0, returns: 2, help: "Image slot shown now and number of slots ()->(active,count)" },
    SoftInterruptFunction { no: 3, name: "set_page",    arguments: 1, returns: 0, help: "Show image slot n (n)->()" },
    SoftInterruptFunction { no: 4, name: "led",         arguments: 2, returns: 0, help: "Switch LED n (1-4) on (1) or off (0) (n,on)->()" },
    SoftInterruptFunction { no: 5, name: "set_text",    arguments: 2, returns: 0, help: "Show a number as the text of page node n (node,value)->()" },
    SoftInterruptFunction { no: 6, name: "report",      arguments: 2, returns: 0, help: "Send a reading to the server as sensor vm<id> (id,value)->()" },
    SoftInterruptFunction { no: 7, name: "uptime",      arguments: 0, returns: 1, help: "Seconds since the device started, wraps at 32767 ()->(s)" },
    SoftInterruptFunction { no: 8, name: "gps",         arguments: 0, returns: 2, help: "GPS satellites tracked and whether there is a fix ()->(sats,fix)" },
    SoftInterruptFunction { no: 9, name: "set_visible", arguments: 2, returns: 0, help: "Show (1) or hide (0) page node n (node,on)->()" },
    SoftInterruptFunction { no: 10, name: "status",     arguments: 1, returns: 1, help: "Device status by id, 0 if the board does not know it (id)->(value)" },
    SoftInterruptFunction { no: 11, name: "has",        arguments: 1, returns: 1, help: "Whether the board knows status id: 1 or 0 (id)->(known)" },
    SoftInterruptFunction { no: 12, name: "value",      arguments: 1, returns: 1, help: "Number k the server set (k)->(value)" },
    SoftInterruptFunction { no: 13, name: "str_clear",  arguments: 1, returns: 0, help: "Empty text slot s (s)->()" },
    SoftInterruptFunction { no: 14, name: "str_add",    arguments: 2, returns: 0, help: "Append text slot t to text slot s (s,t)->()" },
    SoftInterruptFunction { no: 15, name: "str_num",    arguments: 2, returns: 0, help: "Append a number to text slot s (s,value)->()" },
    SoftInterruptFunction { no: 16, name: "str_char",   arguments: 2, returns: 0, help: "Append the character with code c to text slot s, 32 = space (s,c)->()" },
    SoftInterruptFunction { no: 17, name: "str_len",    arguments: 1, returns: 1, help: "Characters in text slot s (s)->(len)" },
    SoftInterruptFunction { no: 18, name: "str_to_num", arguments: 1, returns: 1, help: "The number text slot s starts with, 0 if none (s)->(value)" },
    SoftInterruptFunction { no: 19, name: "str_eq",     arguments: 2, returns: 1, help: "Whether text slots a and b are equal: 1 or 0 (a,b)->(equal)" },
    SoftInterruptFunction { no: 20, name: "show",       arguments: 2, returns: 0, help: "Show text slot s as the text of page node n (node,s)->()" },
    SoftInterruptFunction { no: 21, name: "send",       arguments: 2, returns: 0, help: "Send text slot s to the server as text output id (id,s)->()" },
    SoftInterruptFunction { no: 22, name: "str_new",    arguments: 1, returns: 1, help: "1 once after the server changed text slot s, else 0 (s)->(changed)" },
    SoftInterruptFunction { no: 23, name: "layout",        arguments: 1, returns: 1, help: "Switch to text layout n (a #layout block): its slots start empty, the base stays; 1 if switched (n)->(ok)" },
    SoftInterruptFunction { no: 24, name: "layout_active", arguments: 0, returns: 1, help: "The text layout in use, -1 = none ()->(n)" },
    SoftInterruptFunction { no: 25, name: "text_used",     arguments: 0, returns: 1, help: "Bytes the current text slots reserve ()->(bytes)" },
    SoftInterruptFunction { no: 26, name: "text_budget",   arguments: 0, returns: 1, help: "Bytes of text memory this board allows ()->(bytes)" },
];
