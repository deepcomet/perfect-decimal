import { parentPort, workerData } from "node:worker_threads"

function verifyChunk(start, end, decimalPlaces, progressBuffer, workerId) {
  const multiplier = Math.pow(10, decimalPlaces)
  const totalInChunk = end - start
  const progressArray = new Int32Array(progressBuffer)
  const batchSize = 10000000
  let batchProgress = 0

  for (let i = start; i < end; i++) {
    const intPart = (i / multiplier) | 0
    const fracPart = i % multiplier
    const value = intPart + (fracPart / multiplier)

    const serialized = Number(value.toFixed(decimalPlaces))
    const deserialized = JSON.parse(JSON.stringify(serialized))

    if (serialized !== deserialized || serialized.toFixed(decimalPlaces) !== value.toFixed(decimalPlaces)) {
      parentPort.postMessage({ error: true, value, serialized, deserialized })
      return
    }

    batchProgress++
    if (batchProgress === batchSize) {
      progressArray[workerId] = Math.floor((i - start) / totalInChunk * 1000000)
      Atomics.notify(progressArray, workerId)
      batchProgress = 0
    }
  }

  if (batchProgress > 0) {
    progressArray[workerId] += batchProgress / totalInChunk
    Atomics.notify(progressArray, workerId)
  }

  parentPort.postMessage({ completed: true })
}

verifyChunk(
  workerData.start,
  workerData.end,
  workerData.decimalPlaces,
  workerData.progressBuffer,
  workerData.workerId,
)
