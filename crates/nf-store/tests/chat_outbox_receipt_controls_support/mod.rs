// Reuse the exact frozen independent real receiver commit fixture without changing its source.
include!("../chat_outbox_delivery_support/mod.rs");

impl Fixture {
    pub fn sign_with_receiver_key(&self, receipt: ChatDeliveryReceipt) -> SignedChatReceipt {
        let body = receipt_frame(&receipt);
        let signature = self.bob.device_key.sign(&hash(&body));
        let key = &self
            .membership
            .devices
            .get(&self.bob.public.device)
            .unwrap()
            .key;
        nf_contract::signatures::verify_digest(key, &hash(&body), &signature).unwrap();
        SignedChatReceipt { receipt, signature }
    }
    pub fn sign_with_sender_key(&self, receipt: ChatDeliveryReceipt) -> SignedChatReceipt {
        let body = receipt_frame(&receipt);
        let signature = self.alice.device_key.sign(&hash(&body));
        let key = &self
            .membership
            .devices
            .get(&self.alice.public.device)
            .unwrap()
            .key;
        nf_contract::signatures::verify_digest(key, &hash(&body), &signature).unwrap();
        SignedChatReceipt { receipt, signature }
    }
    pub fn independent_stored_receipt(&self, signed: &SignedChatReceipt) -> Vec<u8> {
        let mut bytes = receipt_frame(&signed.receipt);
        bytes.extend_from_slice(&signed.signature);
        bytes
    }
}
