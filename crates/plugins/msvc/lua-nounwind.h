/* t5 Force-included into every C file built with cl (.cargo/config.toml); only Lua's ldo.c uses
   these names. On Windows x64, longjmp unwinds the stack like an exception unless the jump
   buffer's frame is 0. mlua raises Lua errors from Rust callbacks and hooks (a plugin's bad API
   call, its time limit), so that unwind ran through Rust frames, and with panic = "abort" it
   closed the whole app. With the frame cleared, Lua jumps as it does on Linux, which is what
   mlua is built for. */
#define LUAI_THROW(L, c) longjmp((c)->b, 1)
#define LUAI_TRY(L, c, a) if (setjmp((c)->b) == 0) { ((_JUMP_BUFFER *)&(c)->b)->Frame = 0; a }
#define luai_jmpbuf jmp_buf
