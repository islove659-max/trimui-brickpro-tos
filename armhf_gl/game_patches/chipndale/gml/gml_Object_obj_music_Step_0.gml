var focused;
focused=false;
if (instance_exists(obj_menu_pointer1)) focused=(obj_menu_pointer1.action_step==3);
if (instance_exists(obj_menu_pointer)) focused=(obj_menu_pointer.action_step==4);
if (focused && (keyboard_check_pressed(global.p1_a) || keyboard_check_pressed(global.p1_left) || keyboard_check_pressed(global.p1_right)))
{
 if (keyboard_check_pressed(global.p1_left)) global.music_off=(global.music_off+1) mod 3;
 event_perform(ev_mouse,ev_left_press);
}
