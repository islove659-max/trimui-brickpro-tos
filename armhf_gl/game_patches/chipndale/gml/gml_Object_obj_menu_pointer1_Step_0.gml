if (keyboard_check_pressed(global.p1_down)) action_step=(action_step+1) mod 4;
else if (keyboard_check_pressed(global.p1_up)) action_step=(action_step+3) mod 4;
if (action_step==3) { x=obj_music.x-24; y=obj_music.y+8; }
else { x=530; y=248+50*action_step; }
if (action_step != 3) {
if (keyboard_check_pressed(global.p1_a))
{
    if (action_step == 2)
    {
        game_end();
    }
    else if (action_step == 0)
    {
        with (obj_menu_new)
        {
            event_perform(ev_mouse, ev_left_press);
        }
        instance_destroy();
    }
    else if (action_step == 1)
    {
        with (obj_menu_cont)
        {
            event_perform(ev_mouse, ev_left_press);
        }
    }
}

}
