use r#macro::EnumVariants;

// Simple enum without fields (existing functionality)
#[derive(Debug, Clone, PartialEq, EnumVariants)]
enum Color {
    Red,
    Green,
    Blue,
}

// Nested enum
#[derive(Debug, Clone, PartialEq, EnumVariants)]
enum Size {
    Small,
    Medium,
    Large,
}

// Enum with nested variants (new functionality)
#[derive(Debug, Clone, PartialEq, EnumVariants)]
enum Item {
    SimpleItem,
    ColoredItem(Color),
    SizedItem(Size),
}

#[test]
fn test_simple_enum_variants() {
    let colors = Color::variants();
    assert_eq!(colors.len(), 3);
    assert!(colors.contains(&Color::Red));
    assert!(colors.contains(&Color::Green));
    assert!(colors.contains(&Color::Blue));
}

#[test]
fn test_nested_enum_variants() {
    let items = Item::variants();
    
    // Should contain:
    // - SimpleItem
    // - ColoredItem(Red), ColoredItem(Green), ColoredItem(Blue)
    // - SizedItem(Small), SizedItem(Medium), SizedItem(Large)
    // Total: 7 items
    assert_eq!(items.len(), 7);
    
    assert!(items.contains(&Item::SimpleItem));
    assert!(items.contains(&Item::ColoredItem(Color::Red)));
    assert!(items.contains(&Item::ColoredItem(Color::Green)));
    assert!(items.contains(&Item::ColoredItem(Color::Blue)));
    assert!(items.contains(&Item::SizedItem(Size::Small)));
    assert!(items.contains(&Item::SizedItem(Size::Medium)));
    assert!(items.contains(&Item::SizedItem(Size::Large)));
}

#[test]
fn test_mixed_enum_combinations() {
    // Test that we get all expected combinations
    let items = Item::variants();
    
    let simple_items: Vec<_> = items.iter().filter(|item| matches!(item, Item::SimpleItem)).collect();
    let colored_items: Vec<_> = items.iter().filter(|item| matches!(item, Item::ColoredItem(_))).collect();
    let sized_items: Vec<_> = items.iter().filter(|item| matches!(item, Item::SizedItem(_))).collect();
    
    assert_eq!(simple_items.len(), 1);
    assert_eq!(colored_items.len(), 3);
    assert_eq!(sized_items.len(), 3);
}