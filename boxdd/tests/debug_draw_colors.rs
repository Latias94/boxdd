use boxdd::HexColor;

#[test]
fn hex_color_helpers_round_trip_rgb_values() {
    let color = HexColor::from_rgb(0x12, 0x34, 0x56);
    assert_eq!(color.rgb_u32(), 0x123456);
    assert_eq!(color.into_raw(), 0x123456);
    assert_eq!(color.with_alpha(0x7f), 0x7f123456);

    let masked: HexColor = 0xffabcdefu32.into();
    assert_eq!(masked.rgb_u32(), 0xabcdef);
    assert_eq!(masked.into_raw(), 0xabcdef);

    assert_eq!(HexColor::BOX2D_BLUE.rgb_u32(), 0x30AEBF);
    assert_eq!(HexColor::BOX2D_YELLOW.rgb_u32(), 0xFFEE8C);
}
