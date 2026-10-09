if (keyboard_check_pressed(global.p1_down)) action_step=(action_step+1) mod 5;
else if (keyboard_check_pressed(global.p1_up)) action_step=(action_step+4) mod 5;
if (action_step==4) { x=obj_music.x-24; y=obj_music.y+8; }
else { x=530; y=248+50*action_step; }
if (action_step != 4) {
if (keyboard_check_pressed(global.p1_a))
{
    if (action_step == 3)
    {
        if (instance_exists(obj_menu_quit))
        {
            with (obj_menu_quit)
            {
                instance_destroy();
            }
        }
        if (instance_exists(obj_menu_1p))
        {
            with (obj_menu_1p)
            {
                instance_destroy();
            }
        }
        if (instance_exists(obj_menu_2p))
        {
            with (obj_menu_2p)
            {
                instance_destroy();
            }
        }
        if (instance_exists(obj_menu_ctrl))
        {
            with (obj_menu_ctrl)
            {
                instance_destroy();
            }
        }
        obj_title_control.alarm[1] = 1;
        instance_destroy();
    }
    else if (action_step == 0)
    {
        with (obj_menu_1p)
        {
            event_perform(ev_mouse, ev_left_press);
        }
        instance_destroy();
    }
    else if (action_step == 1)
    {
        with (obj_menu_2p)
        {
            event_perform(ev_mouse, ev_left_press);
        }
        instance_destroy();
    }
    else if (action_step == 2)
    {
        with (obj_menu_ctrl)
        {
            event_perform(ev_mouse, ev_left_press);
        }
        instance_destroy();
    }
}

}
