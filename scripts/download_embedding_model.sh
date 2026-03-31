#!/bin/bash
# Download ONNX Embedding Model for Nexa-net
#
# This script downloads a sentence embedding model from HuggingFace
# and converts it to ONNX format for local inference.
#
# Usage:
#   ./scripts/download_embedding_model.sh [model_name]
#
# Default model: all-MiniLM-L6-v2 (384 dimensions, fast and efficient)
# Other options:
#   - all-mpnet-base-v2 (768 dimensions, higher quality)
#   - paraphrase-multilingual-MiniLM-L12-v2 (384 dimensions, multilingual)

set -e

MODEL_NAME="${1:-all-MiniLM-L6-v2}"
MODEL_DIR="models/${MODEL_NAME}"

echo "=== Nexa-net Embedding Model Downloader ==="
echo "Model: ${MODEL_NAME}"
echo "Target directory: ${MODEL_DIR}"
echo ""

# Check if Python and required packages are available
check_python() {
    if ! command -v python3 &> /dev/null; then
        echo "Error: Python 3 is required for model conversion"
        exit 1
    fi

    python3 -c "import transformers, onnx, onnxruntime" 2>/dev/null || {
        echo "Installing required Python packages..."
        pip3 install transformers onnx onnxruntime sentencepiece protobuf
    }
}

# Create model directory
mkdir -p "${MODEL_DIR}"

# Download and convert model
download_model() {
    echo "Downloading model from HuggingFace..."
    
    python3 << EOF
import os
import json
from transformers import AutoTokenizer, AutoModel
import torch
import onnx

model_name = "${MODEL_NAME}"
output_dir = "${MODEL_DIR}"

# Download tokenizer
print(f"Downloading tokenizer for {model_name}...")
tokenizer = AutoTokenizer.from_pretrained(model_name)
tokenizer.save_pretrained(output_dir)

# Download model
print(f"Downloading model {model_name}...")
model = AutoModel.from_pretrained(model_name)

# Export to ONNX
print("Converting to ONNX format...")
dummy_input = tokenizer("test input", return_tensors="pt", padding=True, truncation=True, max_length=512)

onnx_path = os.path.join(output_dir, "model.onnx")
torch.onnx.export(
    model,
    (dummy_input["input_ids"], dummy_input["attention_mask"]),
    onnx_path,
    input_names=["input_ids", "attention_mask"],
    output_names=["output"],
    dynamic_axes={
        "input_ids": {0: "batch", 1: "sequence"},
        "attention_mask": {0: "batch", 1: "sequence"},
        "output": {0: "batch", 1: "sequence"}
    },
    opset_version=14
)

# Verify ONNX model
print("Verifying ONNX model...")
onnx_model = onnx.load(onnx_path)
onnx.checker.check_model(onnx_model)
print("ONNX model is valid!")

# Save model info
info = {
    "model_name": model_name,
    "dimensions": model.config.hidden_size,
    "max_length": 512,
    "source": "HuggingFace",
    "onnx_path": onnx_path
}
with open(os.path.join(output_dir, "model_info.json"), "w") as f:
    json.dump(info, f, indent=2)

print(f"Model saved to {output_dir}")
print(f"Embedding dimensions: {model.config.hidden_size}")
EOF
}

# Main execution
check_python
download_model

echo ""
echo "=== Download Complete ==="
echo "Model files:"
ls -la "${MODEL_DIR}"
echo ""
echo "To use this model in Nexa-net:"
echo "  let embedder = OnnxEmbedder::new("
echo "      PathBuf::from(\"${MODEL_DIR}/model.onnx\"),"
echo "      512"
echo "  );"
echo ""
echo "Or with VectorizerBuilder:"
echo "  let vectorizer = VectorizerBuilder::new()"
echo "      .onnx(PathBuf::from(\"${MODEL_DIR}/model.onnx\"), 512)"
echo "      .build();"