import os from "node:os"
import { join } from "node:path"
import { Worker } from "node:worker_threads"

function verifyRangeParallel(maxInteger, decimalPlaces) {
  const totalNumbers = maxInteger * Math.pow(10, decimalPlaces) - 1
  const numCPUs = os.cpus().length
  const chunkSize = Math.ceil(totalNumbers / numCPUs)

  const zeros = "0".repeat(decimalPlaces)
  const nines = "9".repeat(decimalPlaces)
  console.log(`Validating safe parsing of ${totalNumbers} numbers in range: 0.${zeros} to ${maxInteger - 1}.${nines}`)

  console.time("Verification")

  const progressBuffer = new SharedArrayBuffer(numCPUs * Int32Array.BYTES_PER_ELEMENT)
  const progressArray = new Int32Array(progressBuffer)

  const workers = []
  for (let i = 0; i < numCPUs; i++) {
    const start = i * chunkSize
    const end = Math.min((i + 1) * chunkSize, totalNumbers)

    const worker = new Worker(join(import.meta.dirname, "./safe-decimal-validate-worker.js"), {
      workerData: { start, end, decimalPlaces, progressBuffer, workerId: i },
    })

    worker.on("message", (msg) => {
      if (msg.error) {
        console.error(
          `Error detected at ${msg.value}. Serialized: ${msg.serialized}, Deserialized: ${msg.deserialized}`,
        )
        console.timeEnd("Verification")
        process.exit(1)
      }
      if (msg.completed) {
        console.log(`Worker completed chunk ${start} to ${end}`)
      }
    })

    worker.on("error", console.error)
    worker.on("exit", (code) => {
      if (code !== 0) console.error(`Worker stopped with exit code ${code}`)
    })

    workers.push(worker)
  }

  let lastProgressString = ""
  const progressInterval = setInterval(() => {
    const overallProgress = progressArray.reduce((sum, progress) => sum + progress, 0) / (numCPUs * 1000000)
    const progressString = `Validating: ${(overallProgress * 100).toFixed(4)}% complete...`
    process.stdout.write("\r" + " ".repeat(lastProgressString.length) + "\r" + progressString)
    lastProgressString = progressString
  }, 1000)

  Promise.all(workers.map(worker => new Promise(resolve => worker.on("exit", resolve)))).then(() => {
    clearInterval(progressInterval)
    process.stdout.write("\n")
    console.timeEnd("Verification")
    console.log(`All tests passed: ${totalNumbers} numbers verified`)
    console.log(`100% of the range verified successfully`)
  })
}

const MAX_INTEGER = 1e9
const DECIMAL_PLACES = 6

verifyRangeParallel(MAX_INTEGER, DECIMAL_PLACES)
