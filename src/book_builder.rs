/// Order book builder from incremental updates
///
/// Maintains bid/ask order book using BTreeMap for efficient price level operations.
/// Processes Add/Modify/Delete/Trade messages to keep book state current.

use std::collections::BTreeMap;
use crate::decoder::MessageRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Bid,
    Ask,
}

impl Side {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Side::Bid),
            1 => Some(Side::Ask),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Order {
    pub order_id: u64,
    pub price: u64, // fixed-point
    pub quantity: u32,
    pub side: Side,
}

/// Order book - maintains all orders organized by price level
#[derive(Debug, Clone)]
pub struct OrderBook {
    // Map: price -> quantity at that price (sum of all orders at this level)
    bids: BTreeMap<u64, u32>,
    asks: BTreeMap<u64, u32>,

    // Map: order_id -> full order details
    orders: std::collections::HashMap<u64, Order>,
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: std::collections::HashMap::new(),
        }
    }

    /// Apply a message to the order book
    pub fn apply_message(&mut self, msg: &MessageRef) -> Result<(), String> {
        match msg {
            MessageRef::AddOrder(m) => {
                let order_id = m.order_id;
                let price = m.price;
                let quantity = m.quantity;
                let side = Side::from_u8(m.side).ok_or("Invalid side")?;

                if self.orders.contains_key(&order_id) {
                    return Err(format!("Duplicate order ID: {}", order_id));
                }

                let order = Order {
                    order_id,
                    price,
                    quantity,
                    side,
                };

                // Add to price level
                let level_map = match side {
                    Side::Bid => &mut self.bids,
                    Side::Ask => &mut self.asks,
                };

                *level_map.entry(price).or_insert(0) += quantity;
                self.orders.insert(order_id, order);

                Ok(())
            }

            MessageRef::ModifyOrder(m) => {
                let order_id = m.order_id;
                let new_quantity = m.new_quantity;

                let order = self
                    .orders
                    .get_mut(&order_id)
                    .ok_or_else(|| format!("Order not found: {}", order_id))?;

                let old_qty = order.quantity;
                let level_map = match order.side {
                    Side::Bid => &mut self.bids,
                    Side::Ask => &mut self.asks,
                };

                // Update level quantity
                if let Some(level_qty) = level_map.get_mut(&order.price) {
                    *level_qty = level_qty.saturating_sub(old_qty).saturating_add(new_quantity);
                    if *level_qty == 0 {
                        level_map.remove(&order.price);
                    }
                }

                order.quantity = new_quantity;
                Ok(())
            }

            MessageRef::DeleteOrder(m) => {
                let order_id = m.order_id;

                let order = self
                    .orders
                    .remove(&order_id)
                    .ok_or_else(|| format!("Order not found: {}", order_id))?;

                let level_map = match order.side {
                    Side::Bid => &mut self.bids,
                    Side::Ask => &mut self.asks,
                };

                if let Some(level_qty) = level_map.get_mut(&order.price) {
                    *level_qty = level_qty.saturating_sub(order.quantity);
                    if *level_qty == 0 {
                        level_map.remove(&order.price);
                    }
                }

                Ok(())
            }

            MessageRef::Trade(m) => {
                let buyer_id = m.buyer_order_id;
                let seller_id = m.seller_order_id;
                let qty = m.quantity;

                // Remove or reduce buyer order
                if let Some(order) = self.orders.get_mut(&buyer_id) {
                    order.quantity = order.quantity.saturating_sub(qty);
                    if order.quantity == 0 {
                        let removed = self.orders.remove(&buyer_id).unwrap();
                        self._remove_from_level(&removed);
                    } else {
                        // Update level
                        let level_map = match order.side {
                            Side::Bid => &mut self.bids,
                            Side::Ask => &mut self.asks,
                        };
                        if let Some(lq) = level_map.get_mut(&order.price) {
                            *lq = lq.saturating_sub(qty);
                            if *lq == 0 {
                                level_map.remove(&order.price);
                            }
                        }
                    }
                }

                // Remove or reduce seller order
                if let Some(order) = self.orders.get_mut(&seller_id) {
                    order.quantity = order.quantity.saturating_sub(qty);
                    if order.quantity == 0 {
                        let removed = self.orders.remove(&seller_id).unwrap();
                        self._remove_from_level(&removed);
                    } else {
                        // Update level
                        let level_map = match order.side {
                            Side::Bid => &mut self.bids,
                            Side::Ask => &mut self.asks,
                        };
                        if let Some(lq) = level_map.get_mut(&order.price) {
                            *lq = lq.saturating_sub(qty);
                            if *lq == 0 {
                                level_map.remove(&order.price);
                            }
                        }
                    }
                }

                Ok(())
            }

            MessageRef::Snapshot(snap) => {
                // Clear current book and apply snapshot
                self.bids.clear();
                self.asks.clear();
                self.orders.clear();

                // Add all bid levels
                for level in snap.bid_levels {
                    let price = level.price;
                    let qty = level.quantity;
                    if qty > 0 {
                        self.bids.insert(price, qty);
                    }
                }

                // Add all ask levels
                for level in snap.ask_levels {
                    let price = level.price;
                    let qty = level.quantity;
                    if qty > 0 {
                        self.asks.insert(price, qty);
                    }
                }

                Ok(())
            }
        }
    }

    fn _remove_from_level(&mut self, order: &Order) {
        let level_map = match order.side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };
        if let Some(qty) = level_map.get_mut(&order.price) {
            *qty = qty.saturating_sub(order.quantity);
            if *qty == 0 {
                level_map.remove(&order.price);
            }
        }
    }

    /// Get best bid price and quantity
    pub fn best_bid(&self) -> Option<(u64, u32)> {
        self.bids
            .iter()
            .rev()
            .next()
            .map(|(&price, &qty)| (price, qty))
    }

    /// Get best ask price and quantity
    pub fn best_ask(&self) -> Option<(u64, u32)> {
        self.asks.iter().next().map(|(&price, &qty)| (price, qty))
    }

    /// Get spread (best ask - best bid) in fixed-point units
    pub fn spread(&self) -> Option<u64> {
        match (self.best_bid(), self.best_ask()) {
            (Some((bid, _)), Some((ask, _))) => {
                if bid < ask {
                    Some(ask - bid)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Get market depth: top n levels on each side
    pub fn depth(&self, n: usize) -> BookDepth {
        let bids: Vec<(u64, u32)> = self
            .bids
            .iter()
            .rev()
            .take(n)
            .map(|(&p, &q)| (p, q))
            .collect();

        let asks: Vec<(u64, u32)> = self
            .asks
            .iter()
            .take(n)
            .map(|(&p, &q)| (p, q))
            .collect();

        BookDepth { bids, asks }
    }

    /// Get number of active orders
    pub fn order_count(&self) -> usize {
        self.orders.len()
    }

    /// Get bid side level count
    pub fn bid_levels(&self) -> usize {
        self.bids.len()
    }

    /// Get ask side level count
    pub fn ask_levels(&self) -> usize {
        self.asks.len()
    }
}

#[derive(Debug, Clone)]
pub struct BookDepth {
    pub bids: Vec<(u64, u32)>,
    pub asks: Vec<(u64, u32)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_book() {
        let book = OrderBook::new();
        assert_eq!(book.best_bid(), None);
        assert_eq!(book.best_ask(), None);
        assert_eq!(book.spread(), None);
    }

    #[test]
    fn test_depth() {
        let book = OrderBook::new();
        let depth = book.depth(5);
        assert_eq!(depth.bids.len(), 0);
        assert_eq!(depth.asks.len(), 0);
    }
}
